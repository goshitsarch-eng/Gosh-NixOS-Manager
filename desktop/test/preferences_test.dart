import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:nixos_toolkit/services/preferences.dart';

void main() {
  test(
    'legacy preference format migrates without losing the selected theme',
    () async {
      final dir = await Directory.systemTemp.createTemp('toolkit-pref-');
      addTearDown(() => dir.delete(recursive: true));
      final file = File('${dir.path}/preferences.json');
      await file.writeAsString('{"color_scheme":"dark"}');
      final store = PreferenceStore(path: file.path);
      final preferences = await store.load();
      expect(preferences.themeMode, ThemeMode.dark);
      preferences.width = 900;
      await store.save(preferences);
      final restored = await store.load();
      expect(restored.themeMode, ThemeMode.dark);
      expect(restored.width, 900);
      expect(dir.listSync().whereType<File>(), hasLength(1));
    },
  );
  test('corrupt preference file is preserved before replacing it', () async {
    final dir = await Directory.systemTemp.createTemp('toolkit-pref-');
    addTearDown(() => dir.delete(recursive: true));
    final file = File('${dir.path}/preferences.json');
    await file.writeAsString('invalid');
    final store = PreferenceStore(path: file.path);
    await expectLater(store.load(), throwsFormatException);
    expect(await file.readAsString(), 'invalid');
    await store.save(Preferences());
    final backup = dir.listSync().whereType<File>().firstWhere(
      (f) => f.path.contains('.corrupt-'),
    );
    expect(await backup.readAsString(), 'invalid');
  });
  test(
    'cosmic-only legacy preference remains readable without its toolkit',
    () async {
      final temp = await Directory.systemTemp.createTemp('toolkit-legacy-');
      try {
        final legacy = File('${temp.path}/color_scheme');
        await legacy.writeAsString('dark');
        final store = PreferenceStore(
          path: '${temp.path}/preferences.json',
          legacyPath: legacy.path,
        );
        final prefs = await store.load();
        expect(prefs.themeMode, ThemeMode.dark);
        await Future.wait([store.save(prefs), store.save(prefs)]);
        expect((await store.load()).themeMode, ThemeMode.dark);
        expect(await legacy.readAsString(), 'dark');
        expect(temp.listSync().where((f) => f.path.endsWith('.tmp')), isEmpty);
      } finally {
        await temp.delete(recursive: true);
      }
    },
  );
}
