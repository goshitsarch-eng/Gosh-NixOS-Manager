// Opt-in destructive VM QA. Never include this target in ordinary unit tests.
// Run only with NIXOS_TOOLKIT_LIVE_VM_QA=1 on a backed-up disposable NixOS VM.
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:nixos_toolkit/main.dart' as app;
import 'package:nixos_toolkit/models/configuration.dart';
import 'package:nixos_toolkit/state/app_controller.dart';
import 'package:nixos_toolkit/ui/app_shell.dart';
import 'package:nixos_toolkit/ui/components/settings.dart';
import 'package:nixos_toolkit/ui/screens/configuration_screens.dart';
import 'package:window_manager/window_manager.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  WidgetController.hitTestWarningShouldBeFatal = true;
  const phase = String.fromEnvironment('QA_PHASE', defaultValue: 'ui');
  testWidgets('live NixOS VM: $phase', (tester) async {
    expect(Platform.environment['NIXOS_TOOLKIT_LIVE_VM_QA'], '1');
    expect(File('/etc/NIXOS').existsSync(), isTrue);
    expect(File('/tmp/toolkit-vm-backup-location').existsSync(), isTrue);
    final evidence = <Map<String, dynamic>>[];
    AppController? controller;
    void record(String feature) {
      evidence.add({'feature': feature, 'passed': true});
      // ignore: avoid_print
      print('LIVE PASS: $feature');
    }

    try {
      await app.main([]);
      await tester.pumpAndSettle();
      final c = controller = tester
          .widget<ToolkitApp>(find.byType(ToolkitApp))
          .controller;
      expect(c.error, isNull);
      expect(c.canManage, isTrue);
      final saved = c.configuration!.copy();

      Future<void> settled() async {
        final deadline = DateTime.now().add(const Duration(minutes: 20));
        do {
          await tester.pump(const Duration(milliseconds: 100));
          if (DateTime.now().isAfter(deadline)) {
            fail('Operation timed out: ${c.busy}');
          }
        } while (!c.idle);
        await tester.pumpAndSettle();
      }

      Future<void> tap(Finder target) async {
        if (find.byType(SnackBar).evaluate().isNotEmpty) {
          await tester.pump(const Duration(seconds: 5));
          await tester.pumpAndSettle();
        }
        await tester.ensureVisible(target);
        if (c.idle) {
          await tester.pumpAndSettle();
        } else {
          // Indeterminate progress animates until the operation finishes.
          // Waiting for it to settle would make the Interrupt button vanish.
          await tester.pump(const Duration(milliseconds: 100));
        }
        await tester.tap(target);
        await tester.pump();
      }

      Future<void> page(int number) async {
        await tap(find.byKey(ValueKey('nav-$number')));
        await settled();
        expect(c.page, number);
        expect(c.error, isNull, reason: pageTitles[number]);
        expect(tester.takeException(), isNull);
      }

      if (phase == 'ui') {
        for (var number = 0; number < pageTitles.length; number++) {
          await page(number);
          record('page: ${pageTitles[number]}');
        }
        expect(c.generations, isNotEmpty);
        expect(c.diskUsage?['error'], isNull);
        record('real generation list and store usage');

        await page(0);
        await tap(find.text('Copy instructions'));
        expect(
          (await Clipboard.getData(Clipboard.kTextPlain))?.text,
          contains('selected.nix'),
        );
        record('native clipboard');
        await tap(find.text('Open configuration folder'));
        await settled();
        expect(c.error, isNull);
        record('configuration folder launcher');

        await page(1);
        for (final profile in c.catalog('profiles')) {
          await tap(find.text(profile['name'] as String));
          await tester.pumpAndSettle();
          expect(c.configuration!.profile, profile['id']);
          await c.selectProfile(profile['id'] as String);
          expect(c.profilePreview, isNotEmpty);
          record('select and preview profile: ${profile['id']}');
        }
        await tap(find.text('Clear desktop selection'));
        expect(c.configuration!.profile, isNull);

        await page(2);
        for (final bundle in c.catalog('bundles')) {
          final tile = find.byKey(PageStorageKey('bundle-${bundle['id']}'));
          final toggle = find.descendant(
            of: tile,
            matching: find.byType(Switch),
          );
          await tap(toggle);
          await tester.pumpAndSettle();
          expect(
            c.configuration!.strings('enabled_bundles'),
            contains(bundle['id']),
          );
          await tap(find.text(bundle['name'] as String));
          await tester.pumpAndSettle();
          for (final package in bundle['packages'] as List) {
            final checkbox = find.descendant(
              of: tile,
              matching: find.widgetWithText(
                CheckboxListTile,
                package['display_name'] as String,
              ),
            );
            await tap(checkbox);
            await tester.pumpAndSettle();
            final selections = c.configuration!.value('bundle_packages') as Map;
            expect(selections[bundle['id']], isNot(contains(package['id'])));
            await tap(checkbox);
          }
          await tap(toggle);
          await tester.pumpAndSettle();
          record('bundle and package checkboxes: ${bundle['id']}');
        }

        await page(3);
        await tester.enterText(
          find.byKey(const Key('package-input')),
          'hello, pkgs.jq, hello',
        );
        await tap(find.byKey(const Key('add-packages')));
        await tester.pumpAndSettle();
        expect(
          c.configuration!
              .strings('custom_packages')
              .where((p) => p == 'hello')
              .length,
          1,
        );
        expect(c.configuration!.strings('custom_packages'), contains('jq'));
        await tap(find.byTooltip('Remove hello'));
        expect(
          c.configuration!.strings('custom_packages'),
          isNot(contains('hello')),
        );
        record('package input, duplicate handling, removal');

        await page(4);
        for (final mode in ['Dark', 'Light', 'System']) {
          await tap(find.text(mode));
          await c.preferenceStore.flush();
          record('theme: $mode');
        }
        for (final field in {
          'hostname': 'toolkit-qa',
          'dns_servers': '1.1.1.1, 8.8.8.8',
          'username': 'gosh',
        }.entries) {
          final target = find.byKey(Key('field-.${field.key}'));
          await tester.ensureVisible(target);
          await tester.enterText(target, field.value);
        }
        for (final label in [
          'Virtual machine management',
          'Docker',
          'VirtualBox',
        ]) {
          await tap(find.widgetWithText(CheckboxListTile, label));
        }
        record('identity and user-group forms');

        for (final number in [5, 6, 7]) {
          await page(number);
          final settings = tester
              .widgetList<SettingSwitch>(find.byType(SettingSwitch))
              .toList();
          for (final setting in settings) {
            final finder = find.widgetWithText(SwitchListTile, setting.label);
            final tile = tester.widget<SwitchListTile>(finder);
            if (tile.onChanged == null) continue;
            final before = c.configuration!.value(
              setting.field,
              setting.section,
            );
            await tap(finder);
            await tester.pumpAndSettle();
            expect(
              c.configuration!.value(setting.field, setting.section),
              !(before as bool),
            );
            await tap(finder);
            await tester.pumpAndSettle();
            record('switch: ${setting.field}');
          }
          final choices = tester
              .widgetList<SettingChoice>(find.byType(SettingChoice))
              .toList();
          for (final setting in choices) {
            final parent = find.byWidgetPredicate(
              (widget) =>
                  widget is SettingChoice &&
                  widget.field == setting.field &&
                  widget.section == setting.section,
            );
            final dropdown = find.descendant(
              of: parent,
              matching: find.byType(DropdownButtonFormField<dynamic>),
            );
            for (final option in setting.values.entries) {
              await tap(dropdown);
              await tester.pumpAndSettle();
              await tap(find.text(option.value).last);
              await tester.pumpAndSettle();
              expect(
                c.configuration!.value(setting.field, setting.section),
                option.key,
              );
            }
            record('choice: ${setting.field}');
          }
        }

        await page(6);
        final port = find.byKey(const Key('field-network_config.ssh_port'));
        await tester.ensureVisible(port);
        await tester.enterText(port, '99999');
        expect(c.fieldErrors, isNotEmpty);
        await page(4);
        await page(6);
        expect(tester.widget<TextField>(port).controller!.text, '99999');
        await tester.ensureVisible(port);
        await tester.enterText(port, '2222');
        expect(c.fieldErrors, isEmpty);
        for (final presets
            in tester
                .widgetList<PortPresets>(find.byType(PortPresets))
                .toList()) {
          final parent = find.byWidgetPredicate(
            (widget) => widget is PortPresets && widget.field == presets.field,
          );
          for (final port in presets.ports) {
            await tap(
              find.descendant(
                of: parent,
                matching: find.widgetWithText(FilterChip, '$port'),
              ),
            );
            await tester.pumpAndSettle();
          }
        }
        record('port validation, retained drafts, presets');

        await tap(find.byTooltip('Application menu'));
        await tester.pumpAndSettle();
        await tap(find.text('About NixOS Toolkit'));
        await tester.pumpAndSettle();
        expect(find.text('Version 0.1.0'), findsOneWidget);
        await tap(find.text('Repository'));
        await tester.pumpAndSettle();
        expect(c.error, isNull);
        await tap(find.text('Close'));
        record('About dialog and repository launcher');

        await windowManager.unmaximize();
        await windowManager.setSize(const Size(640, 480));
        await tester.pump(const Duration(seconds: 1));
        await tester.pumpAndSettle();
        await tap(find.byTooltip('Open navigation menu'));
        await tester.pumpAndSettle();
        await tap(find.byKey(const ValueKey('nav-4')));
        await tester.pumpAndSettle();
        expect(c.page, 4);
        expect(tester.takeException(), isNull);
        await windowManager.setSize(const Size(1100, 760));
        await tester.pumpAndSettle();
        record('real minimum-size window and compact navigation');
        c.configuration = saved;
        c.fieldDrafts.clear();
        c.fieldErrors.clear();
        c.dirty = false;
      } else if (phase == 'apply') {
        c.configuration = ConfigurationDto(
          c.catalogs['defaults'] as Map<String, dynamic>,
        );
        c.setValue('custom_packages', ['hello']);
        await page(10);
        await tap(find.text('Dry run'));
        await settled();
        expect(c.error, isNull, reason: c.logs.join('\n'));
        record('real dry build');
        for (final mode in ['Build', 'Test', 'Boot', 'Switch']) {
          final dropdown = find.byType(DropdownButtonFormField<String>);
          await tap(dropdown);
          await tester.pumpAndSettle();
          final label = {
            'Build': 'Build · No activation',
            'Test': 'Test · Temporary activation',
            'Boot': 'Boot · Activate on restart',
            'Switch': 'Switch · Apply now',
          }[mode]!;
          await tap(find.text(label).last);
          await tester.pumpAndSettle();
          await tap(find.text('Apply changes').first);
          await tester.pumpAndSettle();
          await tap(
            find
                    .widgetWithText(AlertDialog, 'Apply configuration?')
                    .evaluate()
                    .isNotEmpty
                ? find.descendant(
                    of: find.byType(AlertDialog),
                    matching: find.text('Apply changes'),
                  )
                : find.text('Apply changes').last,
          );
          await settled();
          expect(c.error, isNull, reason: c.logs.join('\n'));
          expect(c.dirty, isFalse);
          record('real apply and state save: $mode');
        }
        expect(File('/run/current-system/sw/bin/hello').existsSync(), isTrue);
        await page(8);
        expect(c.generations.length, greaterThan(1));
        record('installed package and real generation changes');
      } else if (phase == 'generations') {
        await page(8);
        final newest = c.generations.firstWhere((g) => g['current'] == true);
        final previous = c.generations.firstWhere(
          (g) => (g['number'] as int) < (newest['number'] as int),
        );
        expect(previous['number'], greaterThan(1));
        final running = Link('/run/current-system').resolveSymbolicLinksSync();
        await tap(find.text('Rollback to previous'));
        await tester.pumpAndSettle();
        await tap(find.text('Next boot'));
        await settled();
        expect(c.error, isNull, reason: c.logs.join('\n'));
        expect(
          c.generations.firstWhere((g) => g['current'] == true)['number'],
          previous['number'],
        );
        expect(Link('/run/current-system').resolveSymbolicLinksSync(), running);
        record(
          'rollback to previous for next boot leaves running system intact',
        );
        for (final protected in [previous, newest]) {
          await c.generationAction(protected['number'] as int, delete: true);
          expect(c.error, contains('Refusing to delete'));
        }
        record('real selected and running generations both reject deletion');
        Finder card(int number) => find.ancestor(
          of: find.text('Generation $number'),
          matching: find.byType(Card),
        );
        await tap(
          find.descendant(
            of: card(newest['number'] as int),
            matching: find.text('Activate'),
          ),
        );
        await tester.pumpAndSettle();
        await tap(find.text('Switch now'));
        await settled();
        expect(c.error, isNull, reason: c.logs.join('\n'));
        record('activate selected generation now');
        await tap(
          find.descendant(
            of: card(previous['number'] as int),
            matching: find.text('Delete'),
          ),
        );
        await tester.pumpAndSettle();
        await tap(
          find.descendant(
            of: find.byType(AlertDialog),
            matching: find.text('Delete'),
          ),
        );
        await settled();
        expect(c.error, isNull, reason: c.logs.join('\n'));
        expect(
          c.generations.any((g) => g['number'] == previous['number']),
          isFalse,
        );
        record('delete an inactive QA generation through confirmation dialog');
      } else if (phase == 'failure' || phase == 'cancel') {
        Map<String, String> managedFiles() => {
          for (final file in Directory(
            '/etc/nixos/nixos-toolkit',
          ).listSync(recursive: true).whereType<File>())
            if (file.path.endsWith('.nix')) file.path: file.readAsStringSync(),
        };
        final before = managedFiles();
        final beforeState = await c.helper!.request('ReadState');
        c.setValue('custom_packages', [
          'hello',
          phase == 'cancel' ? 'tree' : 'toolkitQaPackageDoesNotExist',
        ]);
        c.setRebuild('Build');
        if (phase == 'cancel') {
          await page(10);
          await tap(find.text('Apply changes').first);
          await tester.pumpAndSettle();
          await tap(
            find.descendant(
              of: find.byType(AlertDialog),
              matching: find.text('Apply changes'),
            ),
          );
          final deadline = DateTime.now().add(const Duration(seconds: 30));
          while (!c.logs.any(
            (line) => line.contains('Running: nixos-rebuild'),
          )) {
            await tester.pump(const Duration(milliseconds: 100));
            expect(DateTime.now().isBefore(deadline), isTrue);
          }
          await tap(find.text('Interrupt operation'));
          await tester.pump(const Duration(milliseconds: 250));
          await tap(
            find.descendant(
              of: find.byType(AlertDialog),
              matching: find.text('Interrupt'),
            ),
          );
          await settled();
          expect(c.error, contains('interrupted'));
        } else {
          await c.apply();
          expect(c.logs.join('\n'), contains('toolkitQaPackageDoesNotExist'));
        }
        expect(c.error, isNotNull);
        expect(c.dirty, isTrue);
        expect(managedFiles(), before);
        expect(await c.helper!.request('ReadState'), beforeState);
        record('real $phase restores managed files and saved state');
        await c.loadDiskUsage();
        expect(c.error, isNull);
        record(
          'helper lock released and next request succeeds after failed apply',
        );
      } else if (phase == 'services') {
        final serviceNames = serviceGroups.values.expand((group) => group.keys);
        try {
          for (final service in serviceNames) {
            c.setValue(service, true, 'services_config');
          }
          c.setRebuild('Build');
          await c.apply();
          expect(c.error, isNull, reason: c.logs.join('\n'));
          record('real system build with all 21 service options');
          // Scheduled rebuild/GC jobs are built above. Do not race this test's
          // own rebuilds with automatic maintenance timers.
          c.setValue('auto_upgrade', false, 'services_config');
          c.setValue('auto_gc', false, 'services_config');
          c.setRebuild('Switch');
          await c.apply();
          expect(c.error, isNull, reason: c.logs.join('\n'));
          for (final unit in [
            'NetworkManager',
            'systemd-resolved',
            'cups',
            'avahi-daemon',
            'syncthing',
            'docker',
            'libvirtd',
            'postgresql',
            'redis',
            'earlyoom',
          ]) {
            final result = await Process.run('systemctl', [
              'is-active',
              '$unit.service',
            ]);
            expect(
              result.exitCode,
              0,
              reason: '$unit: ${result.stdout} ${result.stderr}',
            );
            record('activated service: $unit');
          }
        } finally {
          c.configuration = saved;
          c.setRebuild('Switch');
          await c.apply();
          expect(c.error, isNull, reason: c.logs.join('\n'));
          record('restore saved configuration after service checks');
        }
      } else if (phase == 'network') {
        try {
          for (final setting in [
            'fail2ban_enabled',
            'tailscale_enabled',
            'wireguard_enabled',
          ]) {
            c.setValue(setting, true, 'network_config');
          }
          c.setValue('allowed_tcp_ports', [18080], 'network_config');
          c.setValue('allowed_udp_ports', [51821], 'network_config');
          c.setValue('wireguard_listen_port', 51821, 'network_config');
          c.setRebuild('Switch');
          await c.apply();
          expect(c.error, isNull, reason: c.logs.join('\n'));
          for (final unit in ['sshd', 'fail2ban', 'tailscaled', 'firewall']) {
            final result = await Process.run('systemctl', [
              'is-active',
              '$unit.service',
            ]);
            expect(
              result.exitCode,
              0,
              reason: '$unit: ${result.stdout} ${result.stderr}',
            );
            record('network service: $unit');
          }
          final wireguard = await Process.run('/run/current-system/sw/bin/wg', [
            '--version',
          ]);
          expect(wireguard.exitCode, 0);
          record(
            'WireGuard tools and custom firewall-port configuration activated',
          );
        } finally {
          c.configuration = saved;
          c.setRebuild('Switch');
          await c.apply();
          expect(c.error, isNull, reason: c.logs.join('\n'));
          record('restore saved configuration after network checks');
        }
      } else if (phase == 'lifecycle') {
        final persisted = await c.helper!.request('ReadState');
        expect(c.configuration!.toJson(), persisted['payload']);
        record('saved configuration loaded on fresh app startup');
        await c.setTheme(ThemeMode.system);
        final originalScheme = await Process.run('gsettings', [
          'get',
          'org.gnome.desktop.interface',
          'color-scheme',
        ]);
        expect(originalScheme.exitCode, 0);
        try {
          for (final scheme in {
            'prefer-dark': Brightness.dark,
            'default': Brightness.light,
          }.entries) {
            final result = await Process.run('gsettings', [
              'set',
              'org.gnome.desktop.interface',
              'color-scheme',
              scheme.key,
            ]);
            expect(result.exitCode, 0);
            await tester.pump(const Duration(seconds: 2));
            await tester.pumpAndSettle();
            expect(
              Theme.of(tester.element(find.byType(Scaffold))).brightness,
              scheme.value,
            );
          }
          record('Follow System receives real GNOME light/dark changes');
        } finally {
          await Process.run('gsettings', [
            'set',
            'org.gnome.desktop.interface',
            'color-scheme',
            originalScheme.stdout.toString().trim(),
          ]);
        }
        await tester.sendKeyEvent(LogicalKeyboardKey.f5);
        await settled();
        expect(c.error, isNull);
        await tap(find.byTooltip('Application menu'));
        await tester.pumpAndSettle();
        await tap(find.text('Refresh'));
        await settled();
        expect(c.error, isNull);
        record('keyboard and menu host refresh');
        c.setValue('hostname', 'toolkit-unsaved-qa');
        await tap(find.byTooltip('Application menu'));
        await tester.pumpAndSettle();
        await tap(find.text('Quit'));
        await tester.pumpAndSettle();
        expect(find.text('Close NixOS Toolkit?'), findsOneWidget);
        await tap(find.text('Cancel'));
        await tester.pumpAndSettle();
        expect(find.byType(ToolkitApp), findsOneWidget);
        expect(c.dirty, isTrue);
        c.configuration = saved;
        c.dirty = false;
        record('unsaved changes prevent accidental quit');
      } else if (phase == 'cleanup') {
        c.configuration = ConfigurationDto(
          c.catalogs['defaults'] as Map<String, dynamic>,
        );
        c.setRebuild('Switch');
        await c.apply();
        expect(c.error, isNull, reason: c.logs.join('\n'));
        expect(File('/run/current-system/sw/bin/hello').existsSync(), isFalse);
        expect(c.configuration!.strings('custom_packages'), isEmpty);
        record(
          'apply empty selection removes QA packages and restores host defaults',
        );
      } else if (phase == 'maintenance') {
        await page(9);
        for (final action in c.catalog('maintenance')) {
          final tile = find.widgetWithText(ListTile, action['name'] as String);
          await tap(find.descendant(of: tile, matching: find.text('Run')));
          await tester.pumpAndSettle();
          await tap(
            find.descendant(
              of: find.byType(AlertDialog),
              matching: find.text('Run'),
            ),
          );
          await settled();
          expect(c.error, isNull, reason: c.logs.join('\n'));
          record('real maintenance: ${action['command']}');
        }
        await tap(find.byTooltip('Refresh store usage'));
        await settled();
        expect(c.error, isNull);
        record('store usage refresh after maintenance');
      } else {
        fail('Unknown QA_PHASE: $phase');
      }
    } finally {
      final destination = Directory(
        Platform.environment['TOOLKIT_QA_EVIDENCE'] ??
            '/tmp/toolkit-live-evidence',
      );
      destination.createSync(recursive: true);
      File(
        '${destination.path}/$phase.json',
      ).writeAsStringSync(const JsonEncoder.withIndent('  ').convert(evidence));
      File('${destination.path}/$phase.log').writeAsStringSync(
        controller?.logs.join('\n') ?? 'App did not initialize',
      );
    }
  }, timeout: const Timeout(Duration(minutes: 90)));
}
