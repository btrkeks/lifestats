import 'package:intl/intl.dart';
import 'package:opennutritracker/core/utils/calc/unit_calc.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_amount.dart';

import 'package:opennutritracker/core/utils/calc/kcal_gauge_calc.dart';
import 'package:opennutritracker/features/quick_log/domain/food_key.dart';
import 'package:opennutritracker/features/quick_log/domain/suggestion_ranking.dart';
import 'package:opennutritracker/features/quick_log/picker/picker_request.dart';
import 'package:opennutritracker/generated/l10n.dart';

const quickLogSnapshotVersion = 1;

sealed class QuickLogSnapshot {
  const QuickLogSnapshot();

  Map<String, Object?> toJson();
}

final class QuickLogNeedsSetup extends QuickLogSnapshot {
  final String title;
  const QuickLogNeedsSetup(this.title);

  @override
  Map<String, Object?> toJson() => {
    'v': quickLogSnapshotVersion,
    'kind': 'needsSetup',
    'title': title,
  };
}

final class QuickLogReady extends QuickLogSnapshot {
  final String profileId;

  final String logicalDay;

  final int validUntilMs;
  final QuickLogStatus status;
  final QuickLogLabels labels;

  final List<QuickLogRow> rows;

  const QuickLogReady({
    required this.profileId,
    required this.logicalDay,
    required this.validUntilMs,
    required this.status,
    required this.labels,
    required this.rows,
  });

  @override
  Map<String, Object?> toJson() => {
    'v': quickLogSnapshotVersion,
    'kind': 'ready',
    'profileId': profileId,
    'logicalDay': logicalDay,
    'validUntilMs': validUntilMs,
    'status': status.toJson(),
    'labels': labels.toJson(),
    'rows': rows.map((row) => row.toJson()).toList(),
  };
}

class QuickLogStatus {
  final String energyValueText;
  final String energyLabelText;
  final double energyFraction;
  final String proteinText;
  final String proteinCompactText;
  final double proteinFraction;

  const QuickLogStatus({
    required this.energyValueText,
    required this.energyLabelText,
    required this.energyFraction,
    required this.proteinText,
    required this.proteinCompactText,
    required this.proteinFraction,
  });

  Map<String, Object?> toJson() => {
    'energyValueText': energyValueText,
    'energyLabelText': energyLabelText,
    'energyFraction': energyFraction,
    'proteinText': proteinText,
    'proteinCompactText': proteinCompactText,
    'proteinFraction': proteinFraction,
  };
}

class QuickLogLabels {
  final String scanProduct, searchFood, undo, emptyTitle, emptyBody;
  const QuickLogLabels({
    required this.scanProduct,
    required this.searchFood,
    required this.undo,
    required this.emptyTitle,
    required this.emptyBody,
  });

  Map<String, Object?> toJson() => {
    'scanProduct': scanProduct,
    'searchFood': searchFood,
    'undo': undo,
    'emptyTitle': emptyTitle,
    'emptyBody': emptyBody,
  };
}

class QuickLogRow {
  final FoodKey foodKey;
  final String title;
  final String amountText;
  final String energyText;
  final String pillText;
  final String chipText;

  final double baseAmount;
  final String unit;

  final PickerRequest picker;

  const QuickLogRow({
    required this.foodKey,
    required this.title,
    required this.amountText,
    required this.energyText,
    required this.pillText,
    required this.chipText,
    required this.baseAmount,
    required this.unit,
    required this.picker,
  });

  Map<String, Object?> toJson() => {
    'foodKey': foodKey.value,
    'title': title,
    'amountText': amountText,
    'energyText': energyText,
    'pillText': pillText,
    'chipText': chipText,
    'baseAmount': baseAmount,
    'unit': unit,
    'picker': picker.toJson(),
  };
}

class QuickLogInputs {
  final String profileId;
  final DateTime now;
  final DateTime today;
  final int dayStartOffsetMinutes;
  final DateTime nextDayStart;
  final double kcalGoal;
  final double kcalEaten;
  final double proteinGoal;
  final double proteinEaten;
  final bool usesKilojoules;
  final bool usesImperialFoodUnits;
  final List<Suggestion> suggestions;
  final PickerAppearance appearance;

  const QuickLogInputs({
    required this.profileId,
    required this.now,
    required this.today,
    required this.dayStartOffsetMinutes,
    required this.nextDayStart,
    required this.kcalGoal,
    required this.kcalEaten,
    required this.proteinGoal,
    required this.proteinEaten,
    required this.usesKilojoules,
    required this.usesImperialFoodUnits,
    required this.suggestions,
    required this.appearance,
  });
}

QuickLogReady buildQuickLogSnapshot(QuickLogInputs inputs, S s) {
  final integer = NumberFormat.decimalPattern(s.localeName);
  final quantityFormat = NumberFormat.decimalPattern(s.localeName)
    ..maximumFractionDigits = 2;
  final gauge = KcalGaugeCalc.of(
    kcalLeft: inputs.kcalGoal - inputs.kcalEaten,
    kcalGoal: inputs.kcalGoal,
  );
  double energy(double kcal) =>
      inputs.usesKilojoules ? UnitCalc.kcalToKj(kcal) : kcal;
  final energyUnit = inputs.usesKilojoules ? s.kjLabel : s.kcalLabel;
  final energyLabel = inputs.usesKilojoules
      ? '$energyUnit ${gauge.isOver ? s.energyTooMuchLabel : s.energyLeftLabel}'
      : (gauge.isOver ? s.kcalTooMuchLabel : s.kcalLeftLabel);
  final eaten = integer.format(inputs.proteinEaten.round());
  final target = integer.format(inputs.proteinGoal.round());
  final rows = inputs.suggestions.map((suggestion) {
    final latest = suggestion.latest;
    final meal = latest.meal;
    final display = displayQuantityOf(
      baseAmount: latest.amount,
      unit: latest.unit,
      meal: meal,
    );
    final unitTokens = pickerUnitsFor(
      meal,
      imperial: inputs.usesImperialFoodUnits,
    );
    // A unit preference may have changed since the intake was recorded.
    PickerQuantity pickerQuantity(double baseAmount, String token) {
      final quantity = displayQuantityOf(
        baseAmount: baseAmount,
        unit: token,
        meal: meal,
      );
      final offered = unitTokens.contains(quantity.unit);
      return PickerQuantity(
        offered ? quantity.quantity : baseAmount,
        offered ? quantity.unit : unitTokens.first,
      );
    }

    final initial = pickerQuantity(latest.amount, latest.unit);
    final recent =
        suggestion.recentAmounts
            .map((intake) => pickerQuantity(intake.amount, intake.unit))
            .toList()
          ..sort(
            (a, b) => (a.quantity * basePerUnit(a.unit, meal)).compareTo(
              b.quantity * basePerUnit(b.unit, meal),
            ),
          );
    final protein = meal.nutriments.proteinsPerUnit;
    final proteinText = integer.format(
      (latest.amount * (protein ?? 0)).round(),
    );
    final kcal = meal.nutriments.energyPerUnit;
    final title = meal.name?.trim();
    return QuickLogRow(
      foodKey: suggestion.key,
      title: title == null || title.isEmpty
          ? s.additionalInfoLabelUnknown
          : title,
      amountText:
          '${quantityFormat.format(display.quantity)} ${_unitLabel(display.unit, s, count: display.quantity)}',
      energyText: kcal == null
          ? s.notAvailableLabel
          : '${integer.format(energy(latest.amount * kcal).round())} $energyUnit',
      pillText: protein == null
          ? s.quickLogUnknownProteinLabel
          : s.quickLogProteinPill(proteinText),
      chipText: protein == null
          ? s.quickLogUnknownProteinLabel
          : s.quickLogProteinChip(proteinText),
      baseAmount: latest.amount,
      unit: latest.unit,
      picker: PickerRequest(
        foodKey: suggestion.key.value,
        title: title == null || title.isEmpty
            ? s.additionalInfoLabelUnknown
            : title,
        proteinPerBase: protein ?? 0,
        proteinKnown: protein != null,
        energyPerBase: kcal == null ? null : energy(kcal),
        usesKilojoules: inputs.usesKilojoules,
        units: unitTokens
            .map(
              (token) => PickerUnit(
                token,
                _unitLabel(token, s),
                basePerUnit(token, meal),
              ),
            )
            .toList(),
        recent: recent,
        initial: initial,
        appearance: inputs.appearance,
      ),
    );
  }).toList();
  return QuickLogReady(
    profileId: inputs.profileId,
    logicalDay: DateFormat('yyyy-MM-dd').format(inputs.today),
    validUntilMs: inputs.nextDayStart.millisecondsSinceEpoch,
    status: QuickLogStatus(
      energyValueText: integer.format(energy(gauge.value).toInt()),
      energyLabelText: energyLabel,
      energyFraction: gauge.fraction,
      proteinText: s.quickLogProteinProgress(eaten, target),
      proteinCompactText: s.quickLogProteinProgressLong(eaten, target),
      proteinFraction: inputs.proteinGoal > 0
          ? (inputs.proteinEaten / inputs.proteinGoal).clamp(0.0, 1.0)
          : 0,
    ),
    labels: QuickLogLabels(
      scanProduct: s.scanProductLabel,
      searchFood: s.quickLogSearchFoodLabel,
      undo: s.quickLogUndoLabel,
      emptyTitle: s.noMealsRecentlyAddedLabel,
      emptyBody: s.quickLogEmptyBody,
    ),
    rows: rows,
  );
}

String addedTextFor(double proteinGrams, S s) => s.quickLogAddedLabel(
  NumberFormat.decimalPattern(s.localeName).format(proteinGrams.round()),
);

String _unitLabel(String token, S s, {double? count}) => switch (token) {
  'ml' => s.milliliterUnit,
  'oz' => s.ozUnit,
  'fl.oz' => s.flOzUnit,
  'serving' => count == null ? s.servingLabel : s.measureUnitServing(count),
  _ => s.gramUnit,
};
