import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:nixos_toolkit/services/helper_session.dart';

void main() {
  late Directory temp;
  late File trace;
  final logs = <String>[];
  HostHelper peer(String scenario, {void Function(String)? onLog}) =>
      HostHelper(
        program: 'dart',
        arguments: ['test/fixtures/helper.dart', scenario, trace.path],
        onLog: onLog ?? logs.add,
      );
  Future<List<String>> requests() async => (await trace.readAsLines())
      .map((line) => (jsonDecode(line) as Map)['type'] as String)
      .toList();
  setUp(() async {
    temp = await Directory.systemTemp.createTemp('toolkit-helper-');
    trace = File('${temp.path}/requests.jsonl');
    logs.clear();
  });
  tearDown(() => temp.delete(recursive: true));

  test(
    'apply streams progress and saves state after successful rebuild',
    () async {
      await peer('success')
          .apply({'hostname': 'desktop', 'last_applied': null}, 'Switch');
      expect(await requests(), ['EnsureDirectories', 'Apply', 'WriteState']);
      expect(logs, ['Rebuilding']);
      final apply =
          jsonDecode((await trace.readAsLines())[1])['payload'] as Map;
      expect(apply['rebuild_type'], 'Switch');
      expect(apply.containsKey('last_applied'), false);
    },
  );
  test('dry build omits directory creation and state write', () async {
    await peer('success').apply({}, 'DryBuild');
    expect(await requests(), ['Apply']);
  });
  test('state save failure is surfaced after rebuild success', () async {
    await expectLater(
      peer('save-failure').apply({}, 'Switch'),
      throwsA(
        isA<HelperFailure>().having(
          (e) => e.message,
          'message',
          contains('state save failed'),
        ),
      ),
    );
  });
  test('failed rebuild never writes saved state', () async {
    await expectLater(
      peer('apply-failure').apply({}, 'Switch'),
      throwsA(isA<HelperFailure>()),
    );
    expect(await requests(), ['EnsureDirectories', 'Apply']);
  });
  test('unexpected response and disconnected helper fail explicitly', () async {
    await expectLater(
      peer('wrong').request('EnsureDirectories'),
      throwsA(isA<HelperFailure>()),
    );
    await expectLater(
      peer('disconnect').request('EnsureDirectories'),
      throwsA(isA<HelperFailure>()),
    );
  });
  test('interrupt closes the protocol peer without success', () async {
    final started = Completer<void>();
    final helper = peer(
      'wait',
      onLog: (_) {
        if (!started.isCompleted) started.complete();
      },
    );
    final result = helper.request('EnsureDirectories');
    final assertion = expectLater(result, throwsA(isA<HelperFailure>()));
    await started.future.timeout(const Duration(seconds: 10));
    helper.cancel();
    await assertion;
  });
  test(
    'oversized response fails before an unbounded line is decoded',
    () async {
      await expectLater(
        peer('oversized').request('EnsureDirectories'),
        throwsA(
          isA<HelperFailure>().having(
            (e) => e.message,
            'message',
            contains('too large'),
          ),
        ),
      );
    },
  );
  test('Flatpak invokes explicit host argv and forwards protocol pipes', () {
    final helper = HostHelper.fromCapabilities({
      'flatpak': true,
      'helper_path': '/run/current-system/sw/bin/nixos-toolkit-helper',
    }, logs.add);
    expect(helper.program, 'flatpak-spawn');
    expect(helper.arguments, [
      '--host',
      '--forward-fd=0',
      '--forward-fd=1',
      '--',
      'pkexec',
      '/run/current-system/sw/bin/nixos-toolkit-helper',
    ]);
  });
}
