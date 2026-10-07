import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;

class Preferences {
  Preferences({
    this.themeMode = ThemeMode.system,
    this.width = 1100,
    this.height = 760,
  });
  ThemeMode themeMode;
  double width;
  double height;

  Map<String, dynamic> toJson() => {
    'color_scheme': themeMode.name,
    'window_width': width,
    'window_height': height,
  };
}

class PreferenceStore {
  PreferenceStore({String? path, String? legacyPath})
    : path = path ?? _defaultPath(),
      legacyPath = legacyPath ?? (path == null ? _legacyPath() : null);
  final String path;
  final String? legacyPath;
  bool _malformed = false;
  Future<void> _pending = Future.value();

  static String _defaultPath() {
    final base =
        Platform.environment['XDG_CONFIG_HOME'] ??
        p.join(Platform.environment['HOME']!, '.config');
    return p.join(base, 'nixos-toolkit', 'preferences.json');
  }

  static String _legacyPath() {
    final base =
        Platform.environment['XDG_CONFIG_HOME'] ??
        p.join(Platform.environment['HOME']!, '.config');
    return p.join(
      base,
      'cosmic',
      'io.github.goshitsarch_eng.NixosToolkit',
      'v1',
      'color_scheme',
    );
  }

  Future<Preferences> load() async {
    final file = File(path);
    if (!await file.exists()) {
      if (legacyPath != null && await File(legacyPath!).exists()) {
        final saved = (await File(
          legacyPath!,
        ).readAsString()).trim().replaceAll('"', '');
        return Preferences(
          themeMode: ThemeMode.values.firstWhere(
            (m) => m.name == saved,
            orElse: () => ThemeMode.system,
          ),
        );
      }
      return Preferences();
    }
    try {
      final decoded = jsonDecode(await file.readAsString());
      if (decoded is! Map<String, dynamic> ||
          (decoded['window_width'] != null &&
              decoded['window_width'] is! num) ||
          (decoded['window_height'] != null &&
              decoded['window_height'] is! num)) {
        throw const FormatException('Invalid preference data');
      }
      final data = decoded;
      return Preferences(
        themeMode: ThemeMode.values.firstWhere(
          (mode) => mode.name == data['color_scheme'],
          orElse: () => ThemeMode.system,
        ),
        width: ((data['window_width'] as num?)?.toDouble() ?? 1100).clamp(
          640,
          4000,
        ),
        height: ((data['window_height'] as num?)?.toDouble() ?? 760).clamp(
          480,
          3000,
        ),
      );
    } on FormatException {
      _malformed = true;
      rethrow;
    }
  }

  Future<void> flush() => _pending;

  Future<void> save(Preferences preferences) {
    final snapshot = jsonEncode(preferences.toJson());
    final next = _pending.then((_) => _write(snapshot));
    _pending = next.catchError((Object _) {});
    return next;
  }

  Future<void> _write(String snapshot) async {
    final file = File(path);
    await file.parent.create(recursive: true);
    if (_malformed && await file.exists()) {
      await file.copy('$path.corrupt-${DateTime.now().microsecondsSinceEpoch}');
      _malformed = false;
    }
    final temp = File('$path.$pid.tmp');
    await temp.writeAsString(snapshot, flush: true);
    await temp.rename(path);
  }
}
