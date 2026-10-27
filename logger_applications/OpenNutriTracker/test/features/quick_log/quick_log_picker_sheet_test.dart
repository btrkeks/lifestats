import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/features/quick_log/picker/picker_request.dart';
import 'package:opennutritracker/features/quick_log/picker/quick_log_picker.dart';

/// Speisequark from the shared fixture: 12 g protein and 67 kcal per 100 g,
/// last logged 250 g, recent 150/200/250/300 g.
Map<String, Object?> _speisequark() {
  final snapshot =
      jsonDecode(
            File('test/fixtures/quick_log_snapshot_v1.json').readAsStringSync(),
          )
          as Map<String, dynamic>;
  return (snapshot['rows'] as List).first['picker'] as Map<String, Object?>;
}

Future<List<PickerChoice>> _pump(
  WidgetTester tester,
  Map<String, Object?> json,
) async {
  final submitted = <PickerChoice>[];
  await tester.pumpWidget(
    QuickLogPickerApp(
      request: PickerRequest.fromJson(json)!,
      onSubmit: (choice) async {
        submitted.add(choice);
        return true;
      },
      onDismiss: () {},
    ),
  );
  await tester.pumpAndSettle();
  return submitted;
}

ChoiceChip _chip(WidgetTester tester, String label) => tester.widget(
  find.ancestor(of: find.text(label), matching: find.byType(ChoiceChip)),
);

void main() {
  testWidgets('opens on the last amount with its chip selected (s6)', (
    tester,
  ) async {
    await _pump(tester, _speisequark());

    expect(find.text('Speisequark'), findsOneWidget);
    expect(find.textContaining('12 g protein'), findsOneWidget);
    expect(find.textContaining('67 kcal'), findsOneWidget);
    expect(find.text('Recent amounts'), findsOneWidget);
    expect(_chip(tester, '250 g').selected, isTrue);
    expect(_chip(tester, '150 g').selected, isFalse);
    expect(find.textContaining('+30'), findsOneWidget);
    expect(find.textContaining('168 kcal'), findsOneWidget);
  });

  testWidgets('typing 180 g previews 21.6 g protein and adds 180 g (s7)', (
    tester,
  ) async {
    final submitted = await _pump(tester, _speisequark());

    await tester.enterText(find.byType(TextField), '180');
    await tester.pump();

    expect(find.textContaining('+21.6'), findsOneWidget);
    expect(find.textContaining('121 kcal'), findsOneWidget);
    for (final label in ['150 g', '200 g', '250 g', '300 g']) {
      expect(_chip(tester, label).selected, isFalse, reason: label);
    }

    await tester.tap(find.text('Add'));
    await tester.pump();
    expect(submitted.single.toJson(), {'baseAmount': 180.0, 'unit': 'g'});
  });

  testWidgets('a recent chip sets the quantity it names', (tester) async {
    final submitted = await _pump(tester, _speisequark());

    await tester.tap(find.text('150 g'));
    await tester.pump();

    expect(find.widgetWithText(TextField, '150'), findsOneWidget);
    expect(find.textContaining('+18'), findsOneWidget);
    await tester.tap(find.text('Add'));
    await tester.pump();
    expect(submitted.single.baseAmount, 150);
  });

  testWidgets('switching the unit converts through its base factor', (
    tester,
  ) async {
    final json = _speisequark()
      ..['units'] = [
        {'token': 'g', 'label': 'g', 'basePerUnit': 1},
        {'token': 'serving', 'label': 'Serving', 'basePerUnit': 250},
      ];
    final submitted = await _pump(tester, json);

    await tester.enterText(find.byType(TextField), '2');
    await tester.tap(find.text('g').last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Serving').last);
    await tester.pumpAndSettle();

    expect(find.textContaining('+60'), findsOneWidget);
    await tester.tap(find.text('Add'));
    await tester.pump();
    expect(submitted.single.toJson(), {'baseAmount': 500.0, 'unit': 'serving'});
  });

  testWidgets('an empty quantity cannot be added', (tester) async {
    final submitted = await _pump(tester, _speisequark());

    await tester.enterText(find.byType(TextField), '');
    await tester.pump();
    await tester.tap(find.text('Add'));
    await tester.pump();

    expect(submitted, isEmpty);
  });
}
