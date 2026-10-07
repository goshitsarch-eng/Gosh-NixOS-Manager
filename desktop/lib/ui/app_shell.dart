import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../state/app_controller.dart';
import 'screens/catalog_screens.dart';
import 'screens/configuration_screens.dart';
import 'screens/operation_screens.dart';

const pageTitles = [
  'Getting Started',
  'Desktop Profiles',
  'Software Bundles',
  'Custom Packages',
  'System Settings',
  'Hardware',
  'Network',
  'Services',
  'Generations',
  'Maintenance',
  'Apply Changes',
];
const pageIcons = [
  Icons.home_outlined,
  Icons.desktop_windows_outlined,
  Icons.inventory_2_outlined,
  Icons.add_box_outlined,
  Icons.tune,
  Icons.memory,
  Icons.network_check,
  Icons.settings_applications_outlined,
  Icons.history,
  Icons.build_outlined,
  Icons.check_circle_outline,
];

class ToolkitApp extends StatelessWidget {
  const ToolkitApp({
    super.key,
    required this.controller,
    required this.onQuit,
    this.navigatorKey,
  });
  final GlobalKey<NavigatorState>? navigatorKey;
  final AppController controller;
  final Future<void> Function() onQuit;
  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: controller,
    builder: (context, _) => MaterialApp(
      title: 'NixOS Toolkit',
      navigatorKey: navigatorKey,
      debugShowCheckedModeBanner: false,
      themeMode: controller.preferences.themeMode,
      theme: _theme(Brightness.light),
      darkTheme: _theme(Brightness.dark),
      home: _Shell(controller: controller, onQuit: onQuit),
    ),
  );
  ThemeData _theme(Brightness brightness) => ThemeData(
    colorScheme: ColorScheme.fromSeed(
      seedColor: const Color(0xff258ea0),
      brightness: brightness,
    ),
    useMaterial3: true,
    visualDensity: VisualDensity.compact,
    snackBarTheme: const SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      width: 480,
    ),
    inputDecorationTheme: const InputDecorationTheme(
      border: OutlineInputBorder(),
    ),
  );
}

class _RefreshIntent extends Intent {
  const _RefreshIntent();
}

class _QuitIntent extends Intent {
  const _QuitIntent();
}

class _Shell extends StatelessWidget {
  const _Shell({required this.controller, required this.onQuit});
  final AppController controller;
  final Future<void> Function() onQuit;

  Widget sidebar(BuildContext context, {bool drawer = false}) => ListView(
    padding: const EdgeInsets.all(12),
    children: [
      Padding(
        padding: const EdgeInsets.fromLTRB(12, 16, 12, 20),
        child: Text(
          'NixOS Toolkit',
          style: Theme.of(context).textTheme.titleLarge,
        ),
      ),
      ...List.generate(
        pageTitles.length,
        (index) => Padding(
          padding: const EdgeInsets.only(bottom: 3),
          child: ListTile(
            key: ValueKey('nav-$index'),
            selected: controller.page == index,
            selectedTileColor: Theme.of(context).colorScheme.secondaryContainer,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(10),
            ),
            leading: Icon(pageIcons[index], size: 20),
            title: Text(pageTitles[index]),
            onTap: () {
              controller.selectPage(index);
              if (drawer) Navigator.pop(context);
            },
          ),
        ),
      ),
    ],
  );

  Widget screen() => switch (controller.page) {
    0 => OnboardingScreen(controller),
    1 => ProfilesScreen(controller),
    2 => BundlesScreen(controller),
    3 => PackagesScreen(controller),
    4 => SystemScreen(controller),
    5 => HardwareScreen(controller),
    6 => NetworkScreen(controller),
    7 => ServicesScreen(controller),
    8 => GenerationsScreen(controller),
    9 => MaintenanceScreen(controller),
    _ => ApplyScreen(controller),
  };

  Future<void> about(BuildContext context) => showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('NixOS Toolkit'),
      content: const Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Version 0.1.0'),
          SizedBox(height: 12),
          Text('Declarative NixOS system management for Linux.'),
          SizedBox(height: 12),
          Text('GPL-3.0-or-later'),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () async {
            try {
              if (!await launchUrl(
                Uri.parse(
                  'https://github.com/goshitsarch-eng/Gosh-NixOS-Manager',
                ),
              )) {
                controller.reportError('Could not open repository');
              }
            } catch (error) {
              controller.reportError(error.toString());
            }
          },
          child: const Text('Repository'),
        ),
        TextButton(
          onPressed: () async {
            try {
              if (!await launchUrl(
                Uri.parse(
                  'https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/issues',
                ),
              )) {
                controller.reportError('Could not open issue tracker');
              }
            } catch (error) {
              controller.reportError(error.toString());
            }
          },
          child: const Text('Report an issue'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Close'),
        ),
      ],
    ),
  );

  @override
  Widget build(BuildContext context) => Shortcuts(
    shortcuts: const {
      SingleActivator(LogicalKeyboardKey.keyR, control: true): _RefreshIntent(),
      SingleActivator(LogicalKeyboardKey.f5): _RefreshIntent(),
      SingleActivator(LogicalKeyboardKey.keyQ, control: true): _QuitIntent(),
    },
    child: Actions(
      actions: {
        _RefreshIntent: CallbackAction<_RefreshIntent>(
          onInvoke: (_) {
            controller.refresh();
            return null;
          },
        ),
        _QuitIntent: CallbackAction<_QuitIntent>(
          onInvoke: (_) {
            onQuit();
            return null;
          },
        ),
      },
      child: Focus(
        autofocus: true,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 820;
            return Scaffold(
              drawer: compact
                  ? Drawer(child: sidebar(context, drawer: true))
                  : null,
              appBar: AppBar(
                title: Text(
                  '${pageTitles[controller.page]}${controller.dirty ? ' •' : ''}',
                ),
                actions: [
                  IconButton(
                    tooltip: 'Refresh host information (Ctrl+R)',
                    onPressed: controller.idle ? controller.refresh : null,
                    icon: const Icon(Icons.refresh),
                  ),
                  PopupMenuButton<String>(
                    tooltip: 'Application menu',
                    onSelected: (value) {
                      if (value == 'about') about(context);
                      if (value == 'refresh') controller.refresh();
                      if (value == 'quit') onQuit();
                    },
                    itemBuilder: (context) => const [
                      PopupMenuItem(
                        value: 'about',
                        child: Text('About NixOS Toolkit'),
                      ),
                      PopupMenuItem(value: 'refresh', child: Text('Refresh')),
                      PopupMenuItem(value: 'quit', child: Text('Quit')),
                    ],
                  ),
                ],
              ),
              body: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (!compact) SizedBox(width: 240, child: sidebar(context)),
                  if (!compact) const VerticalDivider(width: 1),
                  Expanded(
                    child: Column(
                      children: [
                        if (controller.busy != null) ...[
                          const LinearProgressIndicator(),
                          Padding(
                            padding: const EdgeInsets.all(8),
                            child: Text(controller.busy!),
                          ),
                        ],
                        if (controller.error != null)
                          MaterialBanner(
                            content: SelectableText(controller.error!),
                            actions: [
                              TextButton(
                                onPressed: controller.clearError,
                                child: const Text('Dismiss'),
                              ),
                            ],
                          ),
                        Expanded(
                          child: !controller.ready
                              ? Center(
                                  child: controller.idle
                                      ? FilledButton(
                                          onPressed: controller.initialize,
                                          child: const Text(
                                            'Retry opening application',
                                          ),
                                        )
                                      : const CircularProgressIndicator(),
                                )
                              : SingleChildScrollView(
                                  key: PageStorageKey(
                                    'page-${controller.page}',
                                  ),
                                  padding: const EdgeInsets.all(24),
                                  child: Align(
                                    alignment: Alignment.topLeft,
                                    child: ConstrainedBox(
                                      constraints: const BoxConstraints(
                                        maxWidth: 1050,
                                      ),
                                      child: screen(),
                                    ),
                                  ),
                                ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            );
          },
        ),
      ),
    ),
  );
}
