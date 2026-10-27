import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// The picker runs in its own engine, beside the process engine that owns
/// the Hive boxes. Anything it imports that can open storage would make it
/// a second owner, so its whole import graph is checked here.
void main() {
  test('the amount picker cannot reach storage', () {
    const forbidden = [
      'package:opennutritracker/core/utils/locator.dart',
      'package:opennutritracker/core/utils/hive_db_provider.dart',
      '/data_source/',
      'package:hive',
    ];
    final seen = <String>{};
    final offending = <String>[];
    void visit(String library) {
      if (!seen.add(library)) return;
      final path = library.replaceFirst('package:opennutritracker/', 'lib/');
      for (final match in RegExp(
        r'''^(?:import|export)\s+['"]([^'"]+)['"]''',
        multiLine: true,
      ).allMatches(File(path).readAsStringSync())) {
        final target = _resolve(library, match.group(1)!);
        if (forbidden.any(target.contains)) {
          offending.add('$library -> $target');
        }
        if (target.startsWith('package:opennutritracker/')) visit(target);
      }
    }

    visit(
      'package:opennutritracker/features/quick_log/picker/quick_log_picker.dart',
    );
    expect(seen.length, greaterThan(3), reason: 'the walk found no imports');
    expect(offending, isEmpty);
  });
}

String _resolve(String from, String uri) {
  if (uri.contains(':')) return uri;
  return Uri.parse(from).resolve(uri).toString();
}
