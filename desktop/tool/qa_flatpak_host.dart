// Opt-in host mutation test for a backed-up disposable NixOS VM.
// Requires the Flatpak and matching host helper, with normal polkit authentication.
import 'dart:async';
import 'dart:io';

import 'package:nixos_toolkit/services/helper_session.dart';

Future<void> main() async {
  if (Platform.environment['NIXOS_TOOLKIT_LIVE_VM_QA'] != '1' ||
      !File('/etc/NIXOS').existsSync() ||
      !File('/tmp/toolkit-vm-backup-location').existsSync()) {
    throw StateError('Requires an explicitly opted-in, backed-up NixOS VM');
  }
  final host = HostHelper.fromCapabilities({
    'flatpak': true,
    'helper_path': File('/run/current-system/sw/bin/nixos-toolkit-helper')
        .resolveSymbolicLinksSync(),
  }, (_) {});
  final helper = HostHelper(
    program: 'flatpak',
    arguments: [
      'run',
      '--user',
      '--command=${host.program}',
      'io.github.goshitsarch_eng.NixosToolkit',
      ...host.arguments,
    ],
    onLog: stdout.writeln,
  );
  final state =
      (await helper.request('ReadState'))['payload'] as Map<String, dynamic>;
  if ((await helper.request('GetRecovery'))['payload'] != null) {
    throw StateError('Pending host recovery');
  }
  for (final kind in ['ListGenerations', 'GetDiskUsage']) {
    await helper.request(kind);
    stdout.writeln('PASS Flatpak $kind');
  }
  await helper.apply(state, 'DryBuild');
  stdout.writeln('PASS Flatpak DryBuild');
  try {
    final operation = helper.request('RunMaintenance', {
      'command': 'nix-store --verify --check-contents',
    });
    Timer(const Duration(seconds: 2), helper.cancel);
    await operation;
    throw StateError('Verification ended before cancellation');
  } on HelperFailure catch (error) {
    if (!error.toString().toLowerCase().contains('interrupt') &&
        !error.toString().toLowerCase().contains('cancel')) {
      rethrow;
    }
    stdout.writeln('PASS Flatpak cooperative cancellation');
  }
  await helper.apply(state, 'Build');
  stdout.writeln('PASS Flatpak Build and WriteState');
  if ((await helper.request('GetRecovery'))['payload'] != null) {
    throw StateError('Unexpected recovery');
  }
  stdout.writeln('PASS Flatpak session cleanup');
}
