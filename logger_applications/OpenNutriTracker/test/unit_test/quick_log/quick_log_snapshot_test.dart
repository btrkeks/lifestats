import 'dart:convert';
import 'dart:io';
import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_log_snapshot.dart';
import 'package:opennutritracker/features/quick_log/domain/suggestion_ranking.dart';
import 'package:opennutritracker/features/quick_log/picker/picker_request.dart';
import 'package:opennutritracker/generated/l10n.dart';
import '../../helpers/test_l10n.dart';
import 'quick_log_test_data.dart';

const appearance = PickerAppearance(
  localeTag: 'en',
  themeMode: 'system',
  useMaterialYou: false,
  accentColor: null,
);

List<Suggestion> goldenSuggestions() {
  final quark = food(code: '4005500000000', name: 'Speisequark', serving: null);
  final shake = food(
    code: '',
    name: 'Protein Shake',
    serving: 60,
    protein: 40 / 60 * 100,
    kcal: 230 / 60 * 100,
  );
  return rankSuggestions(
    today: DateTime.utc(2026, 10, 27),
    dayStartOffsetMinutes: 0,
    intakes: [
      intake('q150', DateTime.utc(2026, 10, 24, 12), meal: quark, amount: 150),
      intake('q200', DateTime.utc(2026, 10, 25, 12), meal: quark, amount: 200),
      intake('q300', DateTime.utc(2026, 10, 26, 12), meal: quark, amount: 300),
      intake('q250', DateTime.utc(2026, 10, 27, 12), meal: quark),
      intake(
        'shake',
        DateTime.utc(2026, 10, 26, 13),
        meal: shake,
        amount: 60,
        unit: 'serving',
      ),
    ],
  );
}

QuickLogInputs inputs({
  List<Suggestion>? suggestions,
  double eaten = 168,
  double proteinEaten = 30,
  double proteinGoal = 150,
  bool kj = false,
  bool imperial = false,
  PickerAppearance appearance = appearance,
}) => QuickLogInputs(
  profileId: 'default',
  now: DateTime.utc(2026, 10, 27, 12),
  today: DateTime.utc(2026, 10, 27),
  dayStartOffsetMinutes: 0,
  nextDayStart: DateTime.utc(2026, 10, 28),
  kcalGoal: 2400,
  kcalEaten: eaten,
  proteinGoal: proteinGoal,
  proteinEaten: proteinEaten,
  usesKilojoules: kj,
  usesImperialFoodUnits: imperial,
  suggestions: suggestions ?? goldenSuggestions(),
  appearance: appearance,
);

void main() {
  test(
    'v1 snapshot matches shared native fixture and every picker is complete',
    () {
      final expected = jsonDecode(
        File('test/fixtures/quick_log_snapshot_v1.json').readAsStringSync(),
      );
      final snapshot = buildQuickLogSnapshot(inputs(), l10nEn);
      expect(jsonDecode(jsonEncode(snapshot.toJson())), expected);
      expect(snapshot.logicalDay, '2026-10-27');
      expect(
        DateTime.fromMillisecondsSinceEpoch(snapshot.validUntilMs, isUtc: true),
        DateTime.utc(2026, 10, 28),
      );
      for (final row in snapshot.rows) {
        final parsed = PickerRequest.fromJson(
          jsonDecode(jsonEncode(row.picker.toJson())),
        );
        expect(parsed, isNotNull);
        expect(parsed!.foodKey, row.foodKey.value);
        expect(parsed.toJson(), row.picker.toJson());
      }
    },
  );
  test('over goal uses overshoot, full ring and too much label', () {
    final snapshot = buildQuickLogSnapshot(
      inputs(eaten: 2600, proteinEaten: 180),
      l10nEn,
    );
    expect(snapshot.status.energyValueText, '200');
    expect(snapshot.status.energyLabelText, 'kcal too much');
    expect(snapshot.status.energyFraction, 1);
    expect(snapshot.status.proteinText, '180/150 g');
    expect(snapshot.status.proteinFraction, 1);
  });
  test('kJ preference converts status, row and picker energy together', () {
    final snapshot = buildQuickLogSnapshot(
      inputs(eaten: 2600, kj: true),
      l10nEn,
    );
    expect(snapshot.status.energyValueText, '836');
    expect(snapshot.status.energyLabelText, 'kJ too much');
    expect(snapshot.rows.first.energyText, '701 kJ');
    final request = snapshot.rows.first.picker;
    expect(request.usesKilojoules, isTrue);
    expect(previewFor(request, '180', 'g')!.energy, 505);
  });
  test(
    'unknown nutrition has a bare plus and nullable energy in the picker',
    () {
      final suggestions = rankSuggestions(
        today: DateTime(2026, 10, 27),
        dayStartOffsetMinutes: 0,
        intakes: [
          intake(
            'unknown',
            DateTime(2026, 10, 26, 12),
            meal: food(protein: null, kcal: null),
          ),
        ],
      );
      final row = buildQuickLogSnapshot(
        inputs(suggestions: suggestions),
        l10nEn,
      ).rows.single;
      expect(row.pillText, '+');
      expect(row.chipText, '+');
      expect(row.energyText, l10nEn.notAvailableLabel);
      expect(row.picker.proteinKnown, isFalse);
      expect(row.picker.proteinPerBase, 0);
      expect(row.picker.energyPerBase, isNull);
      expect(PickerRequest.fromJson(row.picker.toJson()), isNotNull);
    },
  );
  test('empty history retains status and localized empty-state actions', () {
    final snapshot = buildQuickLogSnapshot(
      inputs(suggestions: [], eaten: 0, proteinEaten: 0),
      l10nEn,
    );
    expect(snapshot.rows, isEmpty);
    expect(snapshot.status.energyValueText, '2,400');
    expect(snapshot.status.proteinText, '0/150 g');
    expect(snapshot.labels.emptyTitle, 'No meals recently added');
    expect(
      snapshot.labels.emptyBody,
      'Foods you log will show up here with their protein, one tap to add.',
    );
    expect(snapshot.labels.scanProduct, l10nEn.scanProductLabel);
    expect(snapshot.labels.searchFood, 'Search food');
    expect(snapshot.labels.undo, 'Undo');
  });
  test('picker carries locale, theme, Material You and accent verbatim', () {
    const dark = PickerAppearance(
      localeTag: 'de-DE',
      themeMode: 'dark',
      useMaterialYou: true,
      accentColor: 0xff2a8c82,
    );
    final request = buildQuickLogSnapshot(
      inputs(appearance: dark),
      lookupS(const Locale('de')),
    ).rows.first.picker;
    expect(request.appearance.toJson(), dark.toJson());
    expect(
      PickerRequest.fromJson(request.toJson())!.appearance.toJson(),
      dark.toJson(),
    );
  });
  test(
    'a changed imperial preference still produces a valid picker initial',
    () {
      final suggestions = rankSuggestions(
        today: DateTime(2026, 10, 27),
        dayStartOffsetMinutes: 0,
        intakes: [
          intake('oz', DateTime(2026, 10, 26, 12), amount: 113.398, unit: 'oz'),
        ],
      );
      final row = buildQuickLogSnapshot(
        inputs(suggestions: suggestions),
        l10nEn,
      ).rows.single;
      expect(row.amountText, '4 oz');
      expect(row.unit, 'oz');
      expect(row.picker.initial.unit, 'g');
      expect(row.picker.initial.quantity, 113.398);
      expect(PickerRequest.fromJson(row.picker.toJson()), isNotNull);
    },
  );
  test('setup wire version and added label share the localized contract', () {
    expect(QuickLogNeedsSetup(l10nEn.appTitle).toJson(), {
      'v': 1,
      'kind': 'needsSetup',
      'title': l10nEn.appTitle,
    });
    expect(addedTextFor(29.6, l10nEn), 'Added · +30 g');
    final status = buildQuickLogSnapshot(inputs(proteinGoal: 0), l10nEn).status;
    expect(status.proteinFraction, 0);
  });
}
