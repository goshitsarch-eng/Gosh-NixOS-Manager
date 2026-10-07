import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';
import 'package:path/path.dart' as p;

class CoreFailure implements Exception {
  const CoreFailure(this.kind, this.message);
  final String kind;
  final String message;
  @override
  String toString() => message;
}

abstract interface class CoreApi {
  Future<Map<String, dynamic>> call(
    String operation, [
    Map<String, dynamic>? args,
  ]);
}

typedef _CallNative = Pointer<Utf8> Function(Pointer<Uint8>, UintPtr);
typedef _CallDart = Pointer<Utf8> Function(Pointer<Uint8>, int);
typedef _FreeNative = Void Function(Pointer<Utf8>);
typedef _FreeDart = void Function(Pointer<Utf8>);

class NativeCore implements CoreApi {
  NativeCore({String? libraryPath})
    : libraryPath = libraryPath ?? _libraryPath();
  final String libraryPath;

  static String _libraryPath() =>
      Platform.environment['NIXOS_TOOLKIT_CORE_LIBRARY'] ??
      p.join(
        p.dirname(Platform.resolvedExecutable),
        'lib',
        'libnixos_toolkit_core.so',
      );

  @override
  Future<Map<String, dynamic>> call(
    String operation, [
    Map<String, dynamic>? args,
  ]) async {
    final request = jsonEncode({'operation': operation, 'args': ?args});
    final response = await Isolate.run(() => invoke(libraryPath, request));
    if (response['ok'] != true) {
      final error = response['error'] as Map<String, dynamic>;
      throw CoreFailure(error['kind'] as String, error['message'] as String);
    }
    return response['result'] as Map<String, dynamic>;
  }

  static Map<String, dynamic> invoke(String libraryPath, String request) {
    late DynamicLibrary library;
    try {
      library = DynamicLibrary.open(libraryPath);
    } on ArgumentError catch (error) {
      throw CoreFailure('library', 'Could not load the Rust core: $error');
    }
    final call = library.lookupFunction<_CallNative, _CallDart>('toolkit_call');
    final release = library.lookupFunction<_FreeNative, _FreeDart>(
      'toolkit_free',
    );
    final bytes = utf8.encode(request);
    if (bytes.length > 1048576) {
      throw const CoreFailure('protocol', 'The configuration is too large');
    }
    final input = calloc<Uint8>(bytes.length);
    Pointer<Utf8>? output;
    try {
      input.asTypedList(bytes.length).setAll(0, bytes);
      output = call(input, bytes.length);
      if (output == nullptr) {
        throw const CoreFailure('internal', 'The core returned no result');
      }
      return jsonDecode(output.toDartString()) as Map<String, dynamic>;
    } finally {
      if (output != null && output != nullptr) release(output);
      calloc.free(input);
    }
  }
}
