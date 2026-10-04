import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:nixos_toolkit/bridge/core_api.dart';
import 'package:nixos_toolkit/services/preferences.dart';
import 'package:nixos_toolkit/state/app_controller.dart';
import 'package:nixos_toolkit/ui/app_shell.dart';

/// Widget tests use the actual domain API synchronously; isolate dispatch is
/// exercised separately in core_api_test.dart.
class WidgetCore implements CoreApi {
  WidgetCore(this.path);
  final String path;
  @override
  Future<Map<String, dynamic>> call(
    String operation, [
    Map<String, dynamic>? args,
  ]) async {
    final result = NativeCore.invoke(
      path,
      jsonEncode({'operation': operation, 'args': ?args}),
    );
    if (result['ok'] != true) {
      throw CoreFailure(
        result['error']['kind'] as String,
        result['error']['message'] as String,
      );
    }
    return result['result'] as Map<String, dynamic>;
  }
}

void main() {
  late Directory temp;
  late AppController controller;
  setUp(() async {
    temp = await Directory.systemTemp.createTemp('toolkit-widget-');
    controller = AppController(
      core: WidgetCore(Platform.environment['NIXOS_TOOLKIT_CORE_LIBRARY']!),
      preferenceStore: PreferenceStore(path: '${temp.path}/preferences.json'),
    );
    await controller.initialize();
    expect(controller.error, isNull);
  });
  tearDown(() async {
    controller.dispose();
    await temp.delete(recursive: true);
  });

  testWidgets('all pages render and package entry reaches Rust preview', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1100, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      ToolkitApp(controller: controller, onQuit: () async {}),
    );
    for (var page = 0; page < pageTitles.length; page++) {
      await tester.scrollUntilVisible(
        find.byKey(ValueKey('nav-$page')),
        150,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.tap(find.byKey(ValueKey('nav-$page')));
      await tester.pumpAndSettle();
      expect(controller.page, page);
      expect(tester.takeException(), isNull, reason: pageTitles[page]);
    }
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('nav-3')),
      -150,
      scrollable: find.byType(Scrollable).first,
    );
    await tester.tap(find.byKey(const ValueKey('nav-3')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('package-input')),
      'ripgrep, fd',
    );
    await tester.tap(find.byKey(const Key('add-packages')));
    await tester.pumpAndSettle();
    expect(controller.configuration!.strings('custom_packages'), [
      'ripgrep',
      'fd',
    ]);
    expect(controller.dirty, true);
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('nav-10')),
      150,
      scrollable: find.byType(Scrollable).first,
    );
    await tester.tap(find.byKey(const ValueKey('nav-10')));
    await tester.pumpAndSettle();
    expect(controller.preview, contains('ripgrep'));
    expect(controller.preview, contains('fd'));
    final apply = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Apply changes'),
    );
    expect(
      apply.onPressed,
      isNull,
      reason: 'Non-NixOS host must keep privileged actions disabled',
    );
  });

  testWidgets('minimum-size navigation and theme changes remain usable', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(640, 480);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      ToolkitApp(controller: controller, onQuit: () async {}),
    );
    await tester.tap(find.byTooltip('Open navigation menu'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('nav-4')));
    await tester.pumpAndSettle();
    expect(controller.page, 4);
    await tester.runAsync(() async {
      await tester.tap(find.text('Dark'));
      await controller.preferenceStore.flush();
    });
    await tester.pumpAndSettle();
    expect(controller.preferences.themeMode, ThemeMode.dark);
    expect(tester.takeException(), isNull);
    expect(
      jsonDecode(
        File('${temp.path}/preferences.json').readAsStringSync(),
      )['color_scheme'],
      'dark',
    );
  });
  testWidgets(
    'invalid port drafts survive navigation and system theme follows brightness at 2x DPI',
    (tester) async {
      tester.view.physicalSize = const Size(2200, 1520);
      tester.view.devicePixelRatio = 2;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
      await tester.pumpWidget(
        ToolkitApp(controller: controller, onQuit: () async {}),
      );
      await tester.tap(find.byKey(const ValueKey('nav-6')));
      await tester.pumpAndSettle();
      final ports = find.byKey(
        const Key('field-network_config.allowed_tcp_ports'),
      );
      await tester.enterText(ports, '70000');
      await tester.pumpAndSettle();
      expect(
        find.text('Use ports from 1 to 65535, separated by commas'),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('nav-4')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('nav-6')));
      await tester.pumpAndSettle();
      expect(tester.widget<TextField>(ports).controller!.text, '70000');
      expect(controller.dirty, true);
      await tester.enterText(ports, '22, 443');
      await tester.pumpAndSettle();
      expect(controller.fieldErrors, isEmpty);
      expect(
        controller.configuration!.value('allowed_tcp_ports', 'network_config'),
        [22, 443],
      );
      tester.platformDispatcher.platformBrightnessTestValue = Brightness.dark;
      await tester.pumpAndSettle();
      expect(
        Theme.of(tester.element(find.byType(Scaffold))).brightness,
        Brightness.dark,
      );
      tester.platformDispatcher.platformBrightnessTestValue = Brightness.light;
      await tester.pumpAndSettle();
      expect(
        Theme.of(tester.element(find.byType(Scaffold))).brightness,
        Brightness.light,
      );
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets('desktop quit shortcut and application About dialog are wired', (
    tester,
  ) async {
    var quitCount = 0;
    await tester.pumpWidget(
      ToolkitApp(
        controller: controller,
        onQuit: () async {
          quitCount++;
        },
      ),
    );
    await tester.pumpAndSettle();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyQ);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pumpAndSettle();
    expect(quitCount, 1);
    await tester.tap(find.byTooltip('Application menu'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('About NixOS Toolkit'));
    await tester.pumpAndSettle();
    expect(find.text('Version 0.1.0'), findsOneWidget);
    expect(find.text('Repository'), findsOneWidget);
    expect(find.text('Report an issue'), findsOneWidget);
    await tester.tap(find.text('Close'));
    await tester.pumpAndSettle();
    expect(find.text('Version 0.1.0'), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
