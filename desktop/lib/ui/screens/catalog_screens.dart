import 'package:flutter/material.dart';

import '../../state/app_controller.dart';
import '../components/settings.dart';

class ProfilesScreen extends StatelessWidget {
  const ProfilesScreen(this.controller, {super.key});
  final AppController controller;
  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      const Text(
        'Choose one desktop environment. Review its configuration before applying.',
      ),
      const SizedBox(height: 16),
      Wrap(
        spacing: 12,
        runSpacing: 12,
        children: controller.catalog('profiles').map((profile) {
          final selected = controller.configuration!.profile == profile['id'];
          final compatible =
              controller.host['arch'] != 'Aarch64' ||
              profile['arm_compat'] != 'None';
          return SizedBox(
            width: 310,
            child: Card(
              color: selected
                  ? Theme.of(context).colorScheme.secondaryContainer
                  : null,
              child: Semantics(
                selected: selected,
                button: true,
                child: InkWell(
                  borderRadius: BorderRadius.circular(12),
                  onTap: controller.idle && compatible
                      ? () => controller.selectProfile(profile['id'] as String)
                      : null,
                  child: Padding(
                    padding: const EdgeInsets.all(18),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            const Icon(Icons.desktop_windows_outlined),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Text(
                                profile['name'] as String,
                                style: Theme.of(context).textTheme.titleMedium,
                              ),
                            ),
                            if (selected) const Icon(Icons.check_circle),
                          ],
                        ),
                        const SizedBox(height: 10),
                        Text(profile['description'] as String),
                        const SizedBox(height: 8),
                        Text('Display manager: ${profile['display_manager']}'),
                        if (profile['arm_note'] != null)
                          Text(profile['arm_note'] as String),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          );
        }).toList(),
      ),
      const SizedBox(height: 20),
      if (controller.configuration!.profile != null)
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton.icon(
            onPressed: controller.idle
                ? () => controller.selectProfile(null)
                : null,
            icon: const Icon(Icons.clear),
            label: const Text('Clear desktop selection'),
          ),
        ),
      if (controller.profilePreview.isNotEmpty)
        SettingsSection(
          title: 'Profile configuration',
          children: [CodePanel(controller.profilePreview)],
        ),
    ],
  );
}

class BundlesScreen extends StatelessWidget {
  const BundlesScreen(this.controller, {super.key});
  final AppController controller;
  @override
  Widget build(BuildContext context) => Column(
    children: controller.catalog('bundles').map((bundle) {
      final id = bundle['id'] as String;
      final enabled = controller.configuration!
          .strings('enabled_bundles')
          .contains(id);
      final compatible =
          controller.host['arch'] != 'Aarch64' ||
          bundle['arm_compat'] != 'None';
      final selection =
          controller.configuration!.value('bundle_packages') as Map;
      final selected = List<String>.from(
        selection[id] as List? ??
            (bundle['packages'] as List).map((p) => (p as Map)['id']).toList(),
      );
      return Card(
        child: ExpansionTile(
          key: PageStorageKey('bundle-$id'),
          title: Text(bundle['name'] as String),
          subtitle: Text(bundle['description'] as String),
          leading: Switch(
            value: enabled,
            onChanged: compatible && controller.idle
                ? (value) => controller.toggleBundle(bundle, value)
                : null,
          ),
          children: [
            if (bundle['arm_note'] != null)
              Padding(
                padding: const EdgeInsets.all(16),
                child: Text(bundle['arm_note'] as String),
              ),
            ...(bundle['packages'] as List).cast<Map<String, dynamic>>().map(
              (package) => CheckboxListTile(
                title: Text(package['display_name'] as String),
                subtitle: Text(package['id'] as String),
                value: selected.contains(package['id']),
                onChanged: enabled && controller.idle
                    ? (value) => controller.toggleBundlePackage(
                        id,
                        package['id'] as String,
                        value!,
                      )
                    : null,
              ),
            ),
          ],
        ),
      );
    }).toList(),
  );
}

class PackagesScreen extends StatefulWidget {
  const PackagesScreen(this.controller, {super.key});
  final AppController controller;
  @override
  State<PackagesScreen> createState() => _PackagesScreenState();
}

class _PackagesScreenState extends State<PackagesScreen> {
  final input = TextEditingController();
  String? error;
  bool adding = false;
  @override
  void dispose() {
    input.dispose();
    super.dispose();
  }

  Future<void> add() async {
    if (adding || input.text.trim().isEmpty) return;
    setState(() => adding = true);
    try {
      final result = await widget.controller.addPackages(input.text);
      if (!mounted) return;
      input.clear();
      setState(() => error = null);
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(result)));
    } catch (failure) {
      if (mounted) setState(() => error = failure.toString());
    } finally {
      if (mounted) setState(() => adding = false);
    }
  }

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      SettingsSection(
        title: 'Add packages',
        description: 'Use package attributes, pkgs.name, nixpkgs#name, or a Nix package list.',
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                TextField(
                  key: const Key('package-input'),
                  controller: input,
                  maxLines: 3,
                  minLines: 1,
                  enabled: !adding && widget.controller.idle,
                  decoration: InputDecoration(
                    labelText: 'Package names',
                    hintText: 'ripgrep, fd',
                    errorText: error,
                  ),
                  onSubmitted: (_) => add(),
                ),
                const SizedBox(height: 12),
                FilledButton.icon(
                  key: const Key('add-packages'),
                  onPressed: !adding && widget.controller.idle ? add : null,
                  icon: const Icon(Icons.add),
                  label: Text(adding ? 'Adding…' : 'Add packages'),
                ),
              ],
            ),
          ),
        ],
      ),
      SettingsSection(
        title: 'Custom packages',
        description: 'Packages are installed when you apply the configuration.',
        children: [
          if (widget.controller.configuration!
              .strings('custom_packages')
              .isEmpty)
            const Padding(
              padding: EdgeInsets.all(20),
              child: Text('No custom packages selected'),
            ),
          ...widget.controller.configuration!
              .strings('custom_packages')
              .map(
                (package) => ListTile(
                  title: SelectableText(package),
                  subtitle: Text('pkgs.$package'),
                  trailing: IconButton(
                    tooltip: 'Remove $package',
                    icon: const Icon(Icons.close),
                    onPressed: widget.controller.idle
                        ? () => widget.controller.toggleList(
                            'custom_packages',
                            package,
                            false,
                          )
                        : null,
                  ),
                ),
              ),
        ],
      ),
    ],
  );
}
