import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/features/quick_log/picker/picker_request.dart';

Map<String, Object?> fixturePicker() {
  final snapshot =
      jsonDecode(
            File('test/fixtures/quick_log_snapshot_v1.json').readAsStringSync(),
          )
          as Map<String, dynamic>;
  return (snapshot['rows'] as List).first['picker'] as Map<String, Object?>;
}

void main() {
  test('180 g previews 21.6 g protein and 121 kcal', () {
    final request = PickerRequest.fromJson(fixturePicker())!;
    final preview = previewFor(request, '180', 'g')!;
    expect(preview.baseAmount, 180);
    expect(preview.protein, 21.6);
    expect(preview.energy, 121);
    expect(previewFor(request, '180,5', 'g')!.baseAmount, 180.5);
    expect(PickerChoice(preview.baseAmount, 'g').toJson(), {
      'baseAmount': 180.0,
      'unit': 'g',
    });
  });
  test('invalid, unbounded or unknown-unit quantities have no preview', () {
    final request = PickerRequest.fromJson(fixturePicker())!;
    for (final quantity in ['', 'abc', '0', '-1', '10001', 'NaN', 'Infinity']) {
      expect(previewFor(request, quantity, 'g'), isNull, reason: quantity);
    }
    expect(previewFor(request, '100', 'oz'), isNull);
    expect(previewFor(request, '10000', 'g')!.baseAmount, 10000);
  });
  test(
    'serving choices use the transported factor and enforce the base bound',
    () {
      final json = fixturePicker();
      json['units'] = [
        {'token': 'g', 'label': 'g', 'basePerUnit': 1},
        {'token': 'serving', 'label': 'Serving', 'basePerUnit': 250},
      ];
      final request = PickerRequest.fromJson(json)!;
      expect(previewFor(request, '1,5', 'serving')!.baseAmount, 375);
      expect(previewFor(request, '41', 'serving'), isNull);
    },
  );
  test('unknown energy round trips and has no preview energy', () {
    final json = fixturePicker()..['energyPerBase'] = null;
    final request = PickerRequest.fromJson(json)!;
    expect(previewFor(request, '180', 'g')!.energy, isNull);
    expect(request.toJson(), json);
  });
  test('malformed top-level fields are rejected without throwing', () {
    expect(PickerRequest.fromJson(null), isNull);
    expect(PickerRequest.fromJson([]), isNull);
    for (final field in fixturePicker().keys) {
      final json = fixturePicker()..remove(field);
      expect(PickerRequest.fromJson(json), isNull, reason: field);
    }
    for (final (field, value) in <(String, Object?)>[
      ('proteinPerBase', double.nan),
      ('proteinPerBase', -1),
      ('energyPerBase', double.infinity),
      ('proteinKnown', 'true'),
      ('title', ''),
      ('units', []),
      ('units', [null]),
      ('recent', [null]),
      ('initial', {'quantity': 0, 'unit': 'g'}),
      ('initial', {'quantity': 10001, 'unit': 'g'}),
      ('initial', {'quantity': 1, 'unit': 'oz'}),
      ('initial', {'quantity': double.infinity, 'unit': 'g'}),
    ]) {
      expect(
        PickerRequest.fromJson(fixturePicker()..[field] = value),
        isNull,
        reason: '$field=$value',
      );
    }
  });
  test(
    'invalid unit factors and duplicate or unsupported units are rejected',
    () {
      for (final factor in [0, -1, double.nan, double.infinity, '1']) {
        final json = fixturePicker()
          ..['units'] = [
            {'token': 'g', 'label': 'g', 'basePerUnit': factor},
          ];
        expect(PickerRequest.fromJson(json), isNull);
      }
      final json = fixturePicker();
      json['units'] = [...json['units'] as List, ...(json['units'] as List)];
      expect(PickerRequest.fromJson(json), isNull);
      json['units'] = [
        {'token': 'cups', 'label': 'Cup', 'basePerUnit': 250},
      ];
      expect(PickerRequest.fromJson(json), isNull);
    },
  );
  test(
    'all appearance fields are required and malformed values are rejected',
    () {
      final valid = fixturePicker()['appearance'] as Map<String, Object?>;
      for (final field in valid.keys) {
        final json = fixturePicker()
          ..['appearance'] = (Map.of(valid)..remove(field));
        expect(PickerRequest.fromJson(json), isNull, reason: field);
      }
      for (final (field, value) in <(String, Object?)>[
        ('localeTag', ''),
        ('localeTag', 7),
        ('themeMode', 'unknown'),
        ('useMaterialYou', 'true'),
        ('accentColor', -1),
        ('accentColor', 0x100000000),
        ('accentColor', 'teal'),
      ]) {
        final json = fixturePicker()
          ..['appearance'] = (Map.of(valid)..[field] = value);
        expect(PickerRequest.fromJson(json), isNull, reason: '$field=$value');
      }
    },
  );
}
