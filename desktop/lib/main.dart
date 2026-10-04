import 'dart:io';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

import 'bridge/core_api.dart';
import 'services/preferences.dart';
import 'state/app_controller.dart';
import 'ui/app_shell.dart';
import 'ui/screens/operation_screens.dart';

Future<void> main(List<String> arguments) async {
  if (arguments.contains('--diagnose')) {
    try {
      final core = NativeCore();
      final catalogs = await core.call('bootstrap');
      final host = await core.call('probe_host');
      final profile = await core.call('profile_preview', {'id': 'gnome'});
      final preview = await core.call('preview', {
        'state': catalogs['defaults'],
      });
      stdout.writeln(
        jsonEncode({
          'api_version': catalogs['api_version'],
          'profile_count': (catalogs['profiles'] as List).length,
          'bundle_count': (catalogs['bundles'] as List).length,
          'profile_preview_bytes': utf8
              .encode(profile['content'] as String)
              .length,
          'preview_bytes': utf8.encode(preview['content'] as String).length,
          'host': host,
        }),
      );
      await stdout.flush();
      exit(0);
    } catch (error) {
      stderr.writeln('Diagnostics failed: $error');
      await stderr.flush();
      exit(1);
    }
  }

  WidgetsFlutterBinding.ensureInitialized();
  final controller = AppController(
    core: NativeCore(),
    preferenceStore: PreferenceStore(),
  );
  await windowManager.ensureInitialized();
  await windowManager.setMinimumSize(const Size(640, 480));
  await windowManager.setTitle('NixOS Toolkit');
  await windowManager.setSize(
    Size(controller.preferences.width, controller.preferences.height),
  );
  await windowManager.setPreventClose(true);
  runApp(_DesktopLifecycle(controller));
  await windowManager.show();
  await windowManager.focus();
  await controller.initialize();
  await windowManager.setSize(
    Size(controller.preferences.width, controller.preferences.height),
  );
}

class _DesktopLifecycle extends StatefulWidget {
  const _DesktopLifecycle(this.controller);
  final AppController controller;
  @override
  State<_DesktopLifecycle> createState() => _DesktopLifecycleState();
}

class _DesktopLifecycleState extends State<_DesktopLifecycle>
    with WindowListener {
  final navigatorKey = GlobalKey<NavigatorState>();
  bool closing = false;
  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    widget.controller.dispose();
    super.dispose();
  }

  @override
  void onWindowClose() {
    quit();
  }

  Future<void> quit() async {
    if (closing) return;
    closing = true;
    if (!widget.controller.idle) {
      final context = navigatorKey.currentContext;
      if (context != null) {
        await showDialog<void>(
          context: context,
          builder: (context) => AlertDialog(
            title: const Text('Host operation in progress'),
            content: const Text(
              'Wait for the operation to finish, or interrupt it from the Apply page before closing.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: const Text('Keep working'),
              ),
            ],
          ),
        );
      }
      closing = false;
      return;
    }
    if (widget.controller.dirty || !widget.controller.idle) {
      final context = navigatorKey.currentContext;
      if (context == null ||
          !await confirm(
            context,
            'Close NixOS Toolkit?',
            !widget.controller.idle
                ? 'A host operation is in progress. Wait for it to finish before closing.'
                : 'Your selection changes have not been applied. Close without applying?',
            action: 'Close',
          )) {
        closing = false;
        return;
      }
      if (!widget.controller.idle) {
        closing = false;
        return;
      }
    }
    try {
      final size = await windowManager.getSize();
      widget.controller.preferences.width = size.width;
      widget.controller.preferences.height = size.height;
      await widget.controller.preferenceStore.save(
        widget.controller.preferences,
      );
    } catch (error) {
      widget.controller.reportError(
        'Window preferences could not be saved: $error',
      );
    }
    await windowManager.destroy();
    exit(0);
  }

  @override
  Widget build(BuildContext context) => ToolkitApp(
    controller: widget.controller,
    navigatorKey: navigatorKey,
    onQuit: quit,
  );
}
