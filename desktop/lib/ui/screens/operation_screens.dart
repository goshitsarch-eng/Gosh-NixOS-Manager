import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../state/app_controller.dart';
import '../components/settings.dart';

Future<bool> confirm(
  BuildContext context,
  String title,
  String body, {
  String action = 'Continue',
  bool destructive = false,
}) async =>
    await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: SingleChildScrollView(child: Text(body)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            style: destructive
                ? FilledButton.styleFrom(
                    backgroundColor: Theme.of(context).colorScheme.error,
                  )
                : null,
            onPressed: () => Navigator.pop(context, true),
            child: Text(action),
          ),
        ],
      ),
    ) ??
    false;

class OnboardingScreen extends StatelessWidget {
  const OnboardingScreen(this.controller, {super.key});
  final AppController controller;
  @override
  Widget build(BuildContext context) {
    final system = controller.host['system'] as Map<String, dynamic>? ?? {};
    final flake = system['config_mode'] == 'Flake';
    final snippet =
        controller.catalogs[flake ? 'flake_snippet' : 'classic_snippet']
            as String? ??
        '';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          'Manage your NixOS configuration declaratively.',
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 16),
        SettingsSection(
          title: 'System status',
          children: [
            ListTile(
              leading: Icon(
                system['is_nixos'] == true
                    ? Icons.check_circle_outline
                    : Icons.info_outline,
              ),
              title: Text(
                system['is_nixos'] == true
                    ? 'NixOS ${system['nixos_version'] ?? ''}'
                    : 'Preview mode on this Linux host',
              ),
              subtitle: Text(
                'Configuration: ${system['config_mode'] ?? 'Unknown'} · Integration: ${system['integration_status'] ?? 'Unknown'}',
              ),
            ),
            ListTile(
              title: const Text('Host helper'),
              subtitle: Text(
                controller.canManage ? 'Installed and available' : 'Apply, generations and maintenance require a NixOS host with nixos-toolkit-helper installed.',
              ),
            ),
          ],
        ),
        SettingsSection(
          title: 'One-time integration',
          description: 'Add the selected.nix import to your existing NixOS configuration. Your original configuration remains under your control.',
          children: [CodePanel(snippet, height: 360)],
        ),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            FilledButton.icon(
              onPressed: () async {
                try {
                  await Clipboard.setData(ClipboardData(text: snippet));
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('Integration instructions copied'),
                      ),
                    );
                  }
                } catch (failure) {
                  controller.reportError(
                    'Could not copy instructions: $failure',
                  );
                }
              },
              icon: const Icon(Icons.copy),
              label: const Text('Copy instructions'),
            ),
            OutlinedButton.icon(
              onPressed: () async {
                try {
                  if (!await launchUrl(Uri.directory('/etc/nixos'))) {
                    controller.reportError('Could not open /etc/nixos');
                  }
                } catch (failure) {
                  controller.reportError('Could not open /etc/nixos: $failure');
                }
              },
              icon: const Icon(Icons.folder_open),
              label: const Text('Open configuration folder'),
            ),
            OutlinedButton.icon(
              onPressed: controller.idle ? controller.refresh : null,
              icon: const Icon(Icons.refresh),
              label: const Text('Verify integration'),
            ),
          ],
        ),
      ],
    );
  }
}

class GenerationsScreen extends StatelessWidget {
  const GenerationsScreen(this.controller, {super.key});
  final AppController controller;
  Future<void> activate(
    BuildContext context,
    Map<String, dynamic> generation,
  ) async {
    final number = generation['number'] as int;
    final mode = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Activate generation $number?'),
        content: const Text(
          'Switch now applies the configuration immediately. Next boot changes the selected generation for the next startup.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          OutlinedButton(
            onPressed: () => Navigator.pop(context, 'boot'),
            child: const Text('Next boot'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, 'switch'),
            child: const Text('Switch now'),
          ),
        ],
      ),
    );
    if (mode != null) {
      await controller.generationAction(number, activation: mode);
    }
  }

  @override
  Widget build(BuildContext context) {
    final current = controller.generations
        .where((g) => g['current'] == true)
        .firstOrNull;
    final previous =
        controller.generations
            .where(
              (g) =>
                  current != null &&
                  (g['number'] as int) < (current['number'] as int),
            )
            .toList()
          ..sort((a, b) => (b['number'] as int).compareTo(a['number'] as int));
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          spacing: 12,
          children: [
            OutlinedButton.icon(
              onPressed: controller.canManage && controller.idle
                  ? controller.loadGenerations
                  : null,
              icon: const Icon(Icons.refresh),
              label: const Text('Refresh'),
            ),
            FilledButton.icon(
              onPressed:
                  controller.canManage && controller.idle && previous.isNotEmpty
                  ? () => activate(context, previous.first)
                  : null,
              icon: const Icon(Icons.undo),
              label: const Text('Rollback to previous'),
            ),
          ],
        ),
        const SizedBox(height: 16),
        if (!controller.canManage)
          const Text(
            'Generation management is available on a NixOS host with the privileged helper.',
          ),
        if (controller.canManage && controller.generations.isEmpty)
          const Text('No generations were returned.'),
        ...controller.generations.map(
          (g) => Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Generation ${g['number']}${g['current'] == true ? ' · Current' : ''}',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 6),
                  Text(
                    '${g['date']} · NixOS ${g['nixos_version'] ?? 'unknown'} · Kernel ${g['kernel_version'] ?? 'unknown'}',
                  ),
                  if (g['config_rev'] != null)
                    SelectableText(
                      'Configuration revision: ${g['config_rev']}',
                    ),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 12,
                    children: [
                      OutlinedButton(
                        onPressed: controller.idle && g['current'] != true
                            ? () => activate(context, g)
                            : null,
                        child: const Text('Activate'),
                      ),
                      TextButton(
                        onPressed: controller.idle && g['current'] != true
                            ? () async {
                                if (await confirm(
                                  context,
                                  'Delete generation ${g['number']}?',
                                  'This generation will no longer be available for rollback.',
                                  action: 'Delete',
                                  destructive: true,
                                )) {
                                  await controller.generationAction(
                                    g['number'] as int,
                                    delete: true,
                                  );
                                }
                              }
                            : null,
                        child: const Text('Delete'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class MaintenanceScreen extends StatelessWidget {
  const MaintenanceScreen(this.controller, {super.key});
  final AppController controller;
  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      SettingsSection(
        title: 'Nix store',
        children: [
          ListTile(
            title: Text(
              controller.diskUsage == null
                  ? 'Store usage unavailable'
                  : '${controller.diskUsage!['store_size']} · ${controller.diskUsage!['generation_count']} generations',
            ),
            subtitle: controller.diskUsage?['error'] == null
                ? null
                : Text(controller.diskUsage!['error'] as String),
            trailing: IconButton(
              tooltip: 'Refresh store usage',
              onPressed: controller.canManage && controller.idle
                  ? controller.loadDiskUsage
                  : null,
              icon: const Icon(Icons.refresh),
            ),
          ),
        ],
      ),
      ...controller
          .catalog('maintenance')
          .map(
            (action) => Card(
              child: ListTile(
                title: Text(action['name'] as String),
                subtitle: Text(action['description'] as String),
                trailing: OutlinedButton(
                  onPressed: controller.canManage && controller.idle
                      ? () async {
                          if (await confirm(
                            context,
                            '${action['name']}?',
                            action['warning'] as String? ??
                                action['description'] as String,
                            action: 'Run',
                            destructive: action['warning'] != null,
                          )) {
                            await controller.maintenance(
                              action['command'] as String,
                            );
                          }
                        }
                      : null,
                  child: const Text('Run'),
                ),
              ),
            ),
          ),
      if (!controller.canManage)
        const Padding(
          padding: EdgeInsets.all(16),
          child: Text(
            'Maintenance requires the privileged helper on a NixOS host.',
          ),
        ),
      const SizedBox(height: 16),
      CodePanel(controller.logs.join('\n'), height: 260),
    ],
  );
}

class ApplyScreen extends StatelessWidget {
  const ApplyScreen(this.controller, {super.key});
  final AppController controller;
  Future<void> apply(BuildContext context) async {
    if (await confirm(
      context,
      'Apply configuration?',
      'The helper will update the managed Nix modules and run nixos-rebuild ${controller.rebuild.toLowerCase()}. '
          'Review the preview before continuing. Applying an empty selection removes previous managed selections.',
      action: 'Apply changes',
      destructive: true,
    )) {
      await controller.apply();
    }
  }

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      SettingsSection(
        title: 'Configuration preview',
        description: 'Review the files generated from your current selections.',
        children: [CodePanel(controller.preview, height: 380)],
      ),
      Align(
        alignment: Alignment.centerLeft,
        child: OutlinedButton.icon(
          onPressed: controller.idle ? controller.updatePreview : null,
          icon: const Icon(Icons.refresh),
          label: const Text('Refresh preview'),
        ),
      ),
      const SizedBox(height: 20),
      DropdownButtonFormField<String>(
        initialValue: controller.rebuild,
        decoration: const InputDecoration(labelText: 'Rebuild mode'),
        items: const [
          DropdownMenuItem(value: 'Switch', child: Text('Switch · Apply now')),
          DropdownMenuItem(
            value: 'Boot',
            child: Text('Boot · Activate on restart'),
          ),
          DropdownMenuItem(
            value: 'Test',
            child: Text('Test · Temporary activation'),
          ),
          DropdownMenuItem(
            value: 'Build',
            child: Text('Build · No activation'),
          ),
        ],
        onChanged: controller.idle
            ? (value) => controller.setRebuild(value!)
            : null,
      ),
      const SizedBox(height: 20),
      Wrap(
        spacing: 12,
        runSpacing: 12,
        children: [
          FilledButton.icon(
            onPressed: controller.canManage && controller.idle
                ? () => apply(context)
                : null,
            icon: const Icon(Icons.check),
            label: const Text('Apply changes'),
          ),
          OutlinedButton.icon(
            onPressed: controller.canManage && controller.idle
                ? () => controller.apply(dryRun: true)
                : null,
            icon: const Icon(Icons.fact_check_outlined),
            label: const Text('Dry run'),
          ),
          if (!controller.idle)
            TextButton(
              onPressed: () async {
                if (await confirm(
                  context,
                  'Interrupt host operation?',
                  'Interrupting the helper may leave writes or a rebuild in progress. Verify the host configuration before retrying.',
                  action: 'Interrupt',
                  destructive: true,
                )) {
                  controller.helper?.cancel();
                }
              },
              child: const Text('Interrupt operation'),
            ),
        ],
      ),
      if (!controller.canManage)
        const Padding(
          padding: EdgeInsets.symmetric(vertical: 16),
          child: Text(
            'Local preview is available. Applying requires NixOS and the installed host helper.',
          ),
        ),
      const SizedBox(height: 20),
      SettingsSection(
        title: 'Operation log',
        children: [CodePanel(controller.logs.join('\n'), height: 260)],
      ),
    ],
  );
}
