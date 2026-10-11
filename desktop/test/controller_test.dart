import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:nixos_toolkit/bridge/core_api.dart';
import 'package:nixos_toolkit/models/change_review.dart';
import 'package:nixos_toolkit/services/helper_session.dart';
import 'package:nixos_toolkit/services/preferences.dart';
import 'package:nixos_toolkit/state/app_controller.dart';

class TestHost implements CoreApi {
  TestHost(this.native);
  final NativeCore native;
  @override
  Future<Map<String, dynamic>> call(
    String operation, [
    Map<String, dynamic>? args,
  ]) async => operation == 'probe_host'
      ? {'can_manage': true, 'arch': 'X86_64'}
      : native.call(operation, args);
}

class RecordingHelper implements HelperApi {
  RecordingHelper(this.state);
  final Map<String, dynamic> state;
  bool failSave = false;
  bool failRead = false;
  Map<String, dynamic>? recovery;
  int applies = 0;
  int revision = 0;
  bool oldProtocol = false;
  String? rebuild;
  @override
  Future<Map<String, dynamic>> request(
    String type, [
    Map<String, dynamic>? payload,
  ]) async {
    if (failRead) throw const HelperFailure('Could not load existing state');
    if (type == 'GetRecovery') {
      return {'type': 'RecoveryStatus', 'payload': recovery};
    }
    if (type == 'RecoverApply') {
      recovery = null;
      return {'type': 'Ok'};
    }
    if (type == 'ReadStateSnapshot' && oldProtocol) {
      throw const HelperFailure(
        'Invalid request format: unknown variant `ReadStateSnapshot`',
      );
    }
    if (type == 'ReadStateSnapshot') {
      return {
        'type': 'StateSnapshot',
        'payload': {'state': state, 'revision': 'test-$revision'},
      };
    }
    return {'type': 'State', 'payload': state};
  }

  @override
  Future<void> apply(Map<String, dynamic> state, String rebuild) async {
    applies++;
    this.rebuild = rebuild;
    if (state['expected_revision'] != 'test-$revision') {
      throw const HelperFailure(
        'Host configuration changed; nothing was written',
      );
    }
    if (failSave) {
      throw const HelperFailure('Rebuild succeeded, but state save failed');
    }
    if (rebuild != 'DryBuild') {
      this.state
        ..clear()
        ..addAll(Map<String, dynamic>.from(state)..remove('expected_revision'));
      revision++;
    }
  }

  @override
  void cancel() {}
}

void main() {
  final native = NativeCore(
    libraryPath: Platform.environment['NIXOS_TOOLKIT_CORE_LIBRARY']!,
  );
  late Directory temp;
  late AppController controller;
  late RecordingHelper helper;
  setUp(() async {
    temp = await Directory.systemTemp.createTemp('toolkit-controller-');
    helper = RecordingHelper(
      (await native.call('bootstrap'))['defaults'] as Map<String, dynamic>,
    );
    controller = AppController(
      core: TestHost(native),
      helper: helper,
      preferenceStore: PreferenceStore(path: '${temp.path}/prefs.json'),
    );
    await controller.initialize();
    expect(controller.error, isNull);
  });
  tearDown(() async {
    controller.dispose();
    await temp.delete(recursive: true);
  });

  test(
    'failed state save preserves dirty selections; success clears them',
    () async {
      await controller.addPackages('ripgrep');
      helper.failSave = true;
      await controller.apply();
      expect(controller.dirty, true);
      expect(controller.error, contains('state save failed'));
      expect(controller.configuration!.strings('custom_packages'), ['ripgrep']);
      helper.failSave = false;
      await controller.apply();
      expect(controller.dirty, false);
      expect(controller.error, isNull);
    },
  );
  test(
    'dry build preserves dirty state and invalid drafts block host calls',
    () async {
      await controller.addPackages('ripgrep');
      await controller.apply(dryRun: true);
      expect(helper.rebuild, 'DryBuild');
      expect(controller.dirty, true);
      controller.setFieldError('network_config.ssh_port', 'Invalid port');
      await controller.apply();
      expect(helper.applies, 1);
      expect(controller.error, contains('Correct invalid fields'));
    },
  );
  test('WireGuard disabling preserves explicitly selected UDP ports', () async {
    controller.setValue('allowed_udp_ports', [51820, 53], 'network_config');
    controller.setValue('wireguard_enabled', true, 'network_config');
    controller.setValue('wireguard_enabled', false, 'network_config');
    await controller.updatePreview();
    expect(
      controller.configuration!.value('allowed_udp_ports', 'network_config'),
      [51820, 53],
    );
    expect(controller.preview, contains('51820'));
    expect(controller.error, isNull);
  });

  test(
    'unfinished apply survives dismissed errors and blocks another apply',
    () async {
      helper.recovery = {'message': 'An apply did not finish'};
      await controller.refresh();
      controller.clearError();
      await controller.apply();
      expect(helper.applies, 0);
      expect(controller.recovery, isNotNull);
      expect(controller.error, contains('unfinished apply'));
      await controller.recoverApply();
      expect(controller.recovery, isNull);
      expect(controller.error, isNull);
      expect(controller.dirty, isFalse);
    },
  );

  test(
    'failed recovery inspection blocks apply until refresh succeeds',
    () async {
      helper.failRead = true;
      await controller.refresh();
      controller.clearError();
      await controller.apply();
      expect(helper.applies, 0);
      expect(controller.recoveryUnknown, isTrue);
      helper.failRead = false;
      await controller.refresh();
      expect(controller.recoveryUnknown, isFalse);
    },
  );
  test('bundle package deselection reaches authoritative preview', () async {
    final fonts = controller
        .catalog('bundles')
        .firstWhere((b) => b['id'] == 'fonts');
    controller.toggleBundle(fonts, true);
    controller.toggleBundlePackage('fonts', 'inter', false);
    await controller.updatePreview();
    expect(controller.preview, contains('fonts.packages'));
    expect(controller.preview, isNot(contains('pkgs.inter')));
  });
  test('concurrent package submissions cannot duplicate selections', () async {
    await Future.wait([
      controller.addPackages('ripgrep'),
      controller.addPackages('ripgrep'),
    ]);
    expect(controller.configuration!.strings('custom_packages'), ['ripgrep']);
  });
  test('legacy Bluetooth state is visible and can be turned off', () async {
    helper.state['bluetooth_enabled'] = true;
    await controller.initialize();
    expect(
      controller.configuration!.value('bluetooth_enabled', 'hardware_config'),
      true,
    );
    controller.setValue('bluetooth_enabled', false, 'hardware_config');
    await controller.updatePreview();
    expect(controller.configuration!.value('bluetooth_enabled'), false);
    expect(controller.preview, isNot(contains('hardware.bluetooth')));
  });
  test(
    'failed initial state load blocks replacing existing configuration',
    () async {
      helper.failRead = true;
      await controller.initialize();
      controller.clearError();
      await controller.apply();
      expect(helper.applies, 0);
      expect(controller.error, contains('Saved system state was not loaded'));
    },
  );
  test(
    'every profile and bundle selection reaches preview and can be removed',
    () async {
      for (final profile in controller.catalog('profiles')) {
        await controller.selectProfile(profile['id'] as String);
        expect(controller.profilePreview, isNotEmpty);
        await controller.updatePreview();
        expect(controller.preview, contains('/profiles/${profile['id']}.nix'));
      }
      await controller.selectProfile(null);
      expect(controller.profilePreview, isEmpty);
      for (final bundle in controller.catalog('bundles')) {
        final id = bundle['id'] as String;
        controller.toggleBundle(bundle, true);
        await controller.updatePreview();
        expect(controller.preview, contains('/bundles/$id.nix'));
        for (final package in bundle['packages'] as List) {
          controller.toggleBundlePackage(id, package['id'] as String, false);
        }
        expect(
          (controller.configuration!.value('bundle_packages') as Map)[id],
          isEmpty,
        );
        controller.toggleBundle(bundle, false);
        await controller.updatePreview();
        expect(controller.preview, isNot(contains('/bundles/$id.nix')));
      }
      expect(controller.error, isNull);
    },
  );

  test(
    'AI bundle contains installable tools and omits removed entries',
    () async {
      final bundle = controller
          .catalog('bundles')
          .firstWhere((b) => b['id'] == 'ai-tools');
      final packages = bundle['packages'] as List;
      controller.toggleBundle(bundle, true);
      final selection = List<String>.from(
        (controller.configuration!.value('bundle_packages') as Map)['ai-tools']
            as List,
      );
      expect(
        selection,
        containsAll([
          'ollama',
          'claude-code',
          'codex',
          'grok-build',
          'code-cursor',
          'cursor-cli',
          'mistral-vibe',
          'opencode',
          'pi-coding-agent',
        ]),
      );
      expect(selection, hasLength(9));
      for (final id in [
        'chatgpt-desktop',
        'hermes-desktop',
        'muse-code',
        'grok-bot',
        'kimi-code',
      ]) {
        expect(packages.where((p) => p['id'] == id), isEmpty);
        expect(selection, isNot(contains(id)));
      }
      await controller.updatePreview();
      expect(controller.preview, isNot(contains('(throw')));
      expect(
        controller.preview,
        contains('nixpkgs.config.allowUnfree = true;'),
      );
    },
  );

  test('stale drafts stop, then explicit reload discards edits and updates the guard', () async {
    await controller.addPackages('ripgrep');
    helper.state['custom_packages'] = ['fd'];
    helper.revision++;
    await controller.apply();
    expect(controller.error, contains('Host configuration changed'));
    expect(controller.dirty, isTrue);
    expect(helper.state['custom_packages'], ['fd']);
    await controller.reloadSavedSelections();
    expect(controller.error, isNull);
    expect(controller.configuration!.strings('custom_packages'), ['fd']);
    expect(controller.dirty, isFalse);
    expect(controller.changeReview.additions, isEmpty);
    await controller.addPackages('ripgrep');
    await controller.apply();
    expect(controller.error, isNull);
    expect(controller.changeReview.additions, isEmpty);
  });

  test('older helpers cannot bypass the configuration guard', () async {
    helper.oldProtocol = true;
    await controller.initialize();
    expect(controller.canApply, isFalse);
    expect(controller.error, contains('Update the host helper'));
    await controller.apply();
    expect(helper.applies, 0);
  });

  test(
    'review shows removals, network changes, and deduplicated language aliases',
    () async {
      helper.state['custom_packages'] = ['ripgrep'];
      await controller.initialize();
      controller.setValue('custom_packages', ['npm', 'javascript', 'nodejs']);
      controller.setValue('allowed_tcp_ports', [22], 'network_config');
      expect(controller.changeReview.additions, hasLength(1));
      expect(controller.changeReview.removals, ['ripgrep']);
      expect(
        controller.changeReview.text,
        contains('Open TCP ports: Not set → 22'),
      );
      expect(
        controller.changeReview.notes,
        contains('Network changes can interrupt a remote connection.'),
      );
      controller.setRebuild('Build');
      await controller.apply();
      expect(controller.logs.last, contains('running system was not changed'));
      expect(
        rebuildExplanation('Test'),
        contains('does not automatically revert'),
      );
    },
  );
}
