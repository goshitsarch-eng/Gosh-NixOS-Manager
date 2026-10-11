// Opt-in probe against the real privileged helper in a disposable VM.
import 'dart:io';

import 'package:nixos_toolkit/services/helper_session.dart';

Future<void> main() async {
  if (Platform.environment['NIXOS_TOOLKIT_LIVE_VM_QA'] != '1' ||
      !File('/tmp/toolkit-vm-backup-location').existsSync()) {
    throw StateError(
      'Requires explicit live VM QA opt-in and a recovery backup',
    );
  }
  final logs = <String>[];
  final helper = HostHelper.fromCapabilities(
    {
      'helper_path':
          Platform.environment['NIXOS_TOOLKIT_HELPER'] ??
          '/run/current-system/sw/bin/nixos-toolkit-helper',
    },
    (message) {
      logs.add(message);
      stdout.writeln(message);
    },
  );
  final watch = Stopwatch()..start();
  final request = helper.request('RunMaintenance', {
    'command': 'nix-store --verify --check-contents',
  });
  final deadline = DateTime.now().add(const Duration(minutes: 2));
  var authenticated = false;
  while (DateTime.now().isBefore(deadline)) {
    final processes = await Process.run('ps', ['-eo', 'ppid,user,comm']);
    authenticated = '${processes.stdout}'.split('\n').any((line) {
      final fields = line.trim().split(RegExp(r'\s+'));
      return fields.length >= 3 && fields[0] == '$pid' && fields[1] == 'root';
    });
    if (authenticated) break;
    await Future<void>.delayed(const Duration(milliseconds: 100));
  }
  if (!authenticated) throw StateError('Helper did not authenticate');
  watch.reset();
  await Future<void>.delayed(const Duration(seconds: 3));
  stdout.writeln('Requesting interruption at ${watch.elapsed}');
  helper.cancel();
  try {
    final result = await request;
    stdout.writeln('Returned ${result['type']} at ${watch.elapsed}');
    exitCode = 1;
  } catch (error) {
    stdout.writeln('Interrupted at ${watch.elapsed}: $error');
    if (watch.elapsed > const Duration(seconds: 15)) exitCode = 1;
  }
  if (logs.any((line) => line.contains('could not be interrupted'))) {
    exitCode = 1;
  }
}
