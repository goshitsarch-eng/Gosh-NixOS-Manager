import 'dart:convert';
import 'dart:io';

// A protocol peer, confined to the temporary trace path passed by the tests.
Future<void> main(List<String> args) async {
  final scenario = args[0];
  final trace = File(args[1]);
  await for (final line
      in stdin.transform(utf8.decoder).transform(const LineSplitter())) {
    final request = jsonDecode(line) as Map<String, dynamic>;
    await trace.writeAsString('$line\n', mode: FileMode.append);
    final type = request['type'];
    if (type == 'BeginSession') {
      stdout.writeln(jsonEncode({'type': 'Ok'}));
      continue;
    }
    if (scenario == 'disconnect') exit(3);
    if (scenario == 'oversized') {
      stdout.write('x' * 4194305);
      await stdout.flush();
      exit(0);
    }
    if (scenario == 'wait') {
      if (type == 'Cancel') {
        stdout.writeln(
          jsonEncode({
            'type': 'Error',
            'payload': {'message': 'Operation interrupted'},
          }),
        );
        continue;
      }
      stdout.writeln(
        jsonEncode({
          'type': 'Log',
          'payload': {'message': 'Waiting'},
        }),
      );
      continue;
    }
    if (scenario == 'wrong') {
      stdout.writeln(jsonEncode({'type': 'State', 'payload': {}}));
    } else if (scenario == 'save-failure' && type == 'WriteState') {
      stdout.writeln(
        jsonEncode({
          'type': 'Error',
          'payload': {'message': 'Read-only state directory', 'details': null},
        }),
      );
    } else if (type == 'ReadStateSnapshot') {
      stdout.writeln(
        jsonEncode({
          'type': 'StateSnapshot',
          'payload': {
            'state': {'hostname': 'desktop'},
            'revision': 'test-revision',
          },
        }),
      );
    } else if (type == 'Apply') {
      stdout.writeln(
        jsonEncode({
          'type': 'Log',
          'payload': {'level': 'Info', 'message': 'Rebuilding'},
        }),
      );
      stdout.writeln(
        jsonEncode({
          'type': 'ApplyComplete',
          'payload': {
            'success': scenario != 'apply-failure',
            'message': 'Rebuild result',
          },
        }),
      );
    } else {
      stdout.writeln(jsonEncode({'type': 'Ok'}));
    }
  }
}
