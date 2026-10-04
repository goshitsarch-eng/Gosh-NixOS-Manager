import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:nixos_toolkit/bridge/core_api.dart';

void main() {
  final path = Platform.environment['NIXOS_TOOLKIT_CORE_LIBRARY'];
  if (path == null) {
    throw StateError(
      'Build the core and set NIXOS_TOOLKIT_CORE_LIBRARY before testing',
    );
  }
  final core = NativeCore(libraryPath: path);

  test(
    'native bridge loads catalogs and validates legacy configuration',
    () async {
      final bootstrap = await core.call('bootstrap');
      expect(bootstrap['api_version'], 1);
      expect(bootstrap['profiles'], hasLength(13));
      expect(bootstrap['bundles'], hasLength(16));
      final defaults = bootstrap['defaults'] as Map<String, dynamic>;
      expect((await core.call('validate', {'state': defaults}))['valid'], true);
      final preview = await core.call('preview', {
        'state': {
          ...defaults,
          'custom_packages': ['ripgrep'],
        },
      });
      expect(preview['content'], contains('ripgrep'));
    },
  );

  test(
    'bridge transfers Unicode and reports typed failures without a panic',
    () async {
      await expectLater(
        core.call('parse_packages', {'input': '] ['}),
        throwsA(isA<CoreFailure>().having((e) => e.kind, 'kind', 'validation')),
      );
      await expectLater(
        core.call('unknown_operation'),
        throwsA(isA<CoreFailure>().having((e) => e.kind, 'kind', 'protocol')),
      );
      await expectLater(
        core.call('parse_packages', {'input': '🔥'}),
        throwsA(isA<CoreFailure>()),
      );
      final parsed = await core.call('parse_packages', {
        'input': 'pkgs.ripgrep, fd\nbat',
      });
      expect(parsed['added'], ['ripgrep', 'fd', 'bat']);
    },
  );
  test(
    'concurrent bridge calls and large requests retain error contracts',
    () async {
      final results = await Future.wait(
        List.generate(
          24,
          (i) => core.call('parse_packages', {'input': 'custom$i'}),
        ),
      );
      for (var i = 0; i < results.length; i++) {
        expect(results[i]['added'], ['custom$i']);
      }
      expect(
        (await core.call('parse_packages', {'input': ''}))['added'],
        isEmpty,
      );
      await expectLater(
        core.call('parse_packages', {'input': 'a' * 1048576}),
        throwsA(isA<CoreFailure>().having((e) => e.kind, 'kind', 'protocol')),
      );
      final temp = await Directory.systemTemp.createTemp('toolkit-bridge-');
      try {
        final copied = await File(path).copy('${temp.path}/core β.so');
        expect(
          (await NativeCore(libraryPath: copied.path)
              .call('bootstrap'))['api_version'],
          1,
        );
        await expectLater(
          NativeCore(libraryPath: '${temp.path}/missing.so').call('bootstrap'),
          throwsA(isA<CoreFailure>().having((e) => e.kind, 'kind', 'library')),
        );
      } finally {
        await temp.delete(recursive: true);
      }
    },
  );
}
