import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

class HelperFailure implements Exception {
  const HelperFailure(this.message);
  final String message;
  @override
  String toString() => message;
}

abstract interface class HelperApi {
  Future<Map<String, dynamic>> request(
    String type, [
    Map<String, dynamic>? payload,
  ]);
  Future<void> apply(Map<String, dynamic> state, String rebuild);
  void cancel();
}

class HostHelper implements HelperApi {
  HostHelper({
    required this.program,
    required this.arguments,
    required this.onLog,
  });
  final String program;
  final List<String> arguments;
  final void Function(String) onLog;
  Process? _process;
  bool _cancelled = false;
  bool _starting = false;

  factory HostHelper.fromCapabilities(
    Map<String, dynamic> host,
    void Function(String) onLog,
  ) {
    final path = host['helper_path'] as String;
    if (host['flatpak'] == true) {
      return HostHelper(
        program: 'flatpak-spawn',
        arguments: [
          '--host',
          '--forward-fd=0',
          '--forward-fd=1',
          '--',
          'pkexec',
          path,
        ],
        onLog: onLog,
      );
    }
    return HostHelper(program: 'pkexec', arguments: [path], onLog: onLog);
  }

  Future<T> _session<T>(Future<T> Function(_Connection) action) async {
    if (_process != null || _starting) {
      throw const HelperFailure('Another host operation is running');
    }
    _cancelled = false;
    final environment = Map<String, String>.from(Platform.environment)
      ..remove('SHELL');
    _starting = true;
    late Process process;
    try {
      process = await Process.start(
        program,
        arguments,
        environment: environment,
        includeParentEnvironment: false,
        runInShell: false,
      );
    } finally {
      _starting = false;
    }
    _process = process;
    final errors = process.stderr
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .listen((line) => onLog(line));
    final replies = StreamIterator(_boundedLines(process.stdout));
    try {
      final result = await action(_Connection(process, replies, onLog));
      await process.stdin.close();
      final code = await process.exitCode.timeout(const Duration(seconds: 5));
      if (_cancelled) {
        throw const HelperFailure(
          'Operation interrupted; verify the host state before retrying',
        );
      }
      if (code != 0) {
        throw HelperFailure('Host helper exited with status $code');
      }
      return result;
    } finally {
      await replies.cancel();
      await errors.cancel();
      process.kill();
      _process = null;
    }
  }

  @override
  Future<Map<String, dynamic>> request(
    String type, [
    Map<String, dynamic>? payload,
  ]) => _session((connection) => connection.send(type, payload));

  @override
  Future<void> apply(Map<String, dynamic> state, String rebuild) => _session((
    connection,
  ) async {
    if (rebuild != 'DryBuild') await connection.send('EnsureDirectories');
    final payload = Map<String, dynamic>.from(state)
      ..remove('last_applied')
      ..['rebuild_type'] = rebuild;
    final result = await connection.send('Apply', payload);
    if (result['type'] != 'ApplyComplete' ||
        (result['payload'] as Map<String, dynamic>)['success'] != true) {
      throw HelperFailure(
        (result['payload'] as Map<String, dynamic>?)?['message'] as String? ??
            'Apply failed',
      );
    }
    if (rebuild != 'DryBuild') {
      try {
        await connection.send('WriteState', {'state': state});
      } catch (error) {
        throw HelperFailure('Rebuild succeeded, but state save failed: $error');
      }
    }
  });

  @override
  void cancel() {
    _cancelled = true;
    _process?.kill(ProcessSignal.sigterm);
  }
}

class _Connection {
  _Connection(this.process, this.replies, this.onLog);
  final Process process;
  final StreamIterator<String> replies;
  final void Function(String) onLog;

  Future<Map<String, dynamic>> send(
    String type, [
    Map<String, dynamic>? payload,
  ]) async {
    process.stdin.writeln(jsonEncode({'type': type, 'payload': ?payload}));
    await process.stdin.flush();
    while (await replies.moveNext().timeout(const Duration(minutes: 30))) {
      final line = replies.current;
      if (line.length > 4194304) {
        throw const HelperFailure('Host response is too large');
      }
      final response = jsonDecode(line) as Map<String, dynamic>;
      if (response['type'] == 'Log') {
        onLog(
          (response['payload'] as Map<String, dynamic>)['message'] as String,
        );
        continue;
      }
      if (response['type'] == 'Error') {
        final error = response['payload'] as Map<String, dynamic>;
        throw HelperFailure(
          '${error['message']}${error['details'] == null ? '' : ': ${error['details']}'}',
        );
      }
      const expected = {
        'EnsureDirectories': 'Ok',
        'WriteState': 'Ok',
        'ReadState': 'State',
        'GetSystemInfo': 'SystemInfo',
        'Apply': 'ApplyComplete',
        'ListGenerations': 'Generations',
        'RollbackGeneration': 'Ok',
        'DeleteGenerations': 'Ok',
        'RunMaintenance': 'MaintenanceOutput',
        'GetDiskUsage': 'DiskUsage',
      };
      if (expected[type] == null || response['type'] != expected[type]) {
        throw HelperFailure('Unexpected ${response['type']} response to $type');
      }
      return response;
    }
    throw const HelperFailure('Host helper closed before returning a result');
  }
}

// Bound each protocol record while receiving it, before JSON/UTF-8 allocation.
Stream<String> _boundedLines(Stream<List<int>> source) async* {
  final buffer = BytesBuilder(copy: false);
  var length = 0;
  await for (final chunk in source) {
    var start = 0;
    for (var i = 0; i <= chunk.length; i++) {
      if (i != chunk.length && chunk[i] != 10) continue;
      final size = i - start;
      length += size;
      if (length > 4194304) {
        throw const HelperFailure('Host response is too large');
      }
      if (size > 0) buffer.add(chunk.sublist(start, i));
      if (i < chunk.length) {
        yield utf8.decode(buffer.takeBytes());
        length = 0;
      }
      start = i + 1;
    }
  }
  if (length > 0) yield utf8.decode(buffer.takeBytes());
}
