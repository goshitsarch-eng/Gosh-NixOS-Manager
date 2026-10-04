import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:nixos_toolkit/bridge/core_api.dart';
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
  int applies = 0;
  String? rebuild;
  @override
  Future<Map<String, dynamic>> request(
    String type, [
    Map<String, dynamic>? payload,
  ]) async => {'type': 'State', 'payload': state};
  @override
  Future<void> apply(Map<String, dynamic> state, String rebuild) async {
    applies++;
    this.rebuild = rebuild;
    if (failSave) {
      throw const HelperFailure('Rebuild succeeded, but state save failed');
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
      controller.clearError();
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
}
