import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/domain/entity/weight_log_entity.dart';
import 'package:opennutritracker/core/utils/calc/macro_calc.dart';
import 'package:opennutritracker/core/utils/calc/protein_goal_calc.dart';

WeightLogEntity _entry(DateTime date, double kg) =>
    WeightLogEntity(date: date, weightKg: kg);

void main() {
  final today = DateTime(2026, 10, 27);
  DateTime daysAgo(int n) => DateTime(today.year, today.month, today.day - n);

  group('ProteinGoalCalc.referenceWeight', () {
    test('falls back to the profile weight without any weigh-ins', () {
      final ref = ProteinGoalCalc.referenceWeight(
        const [],
        today: today,
        profileWeightKg: 81.0,
      );

      expect(ref.weightKg, 81.0);
      expect(ref.sampleCount, 0);
      expect(ref.isFromWeightLog, isFalse);
    });

    test('falls back to the profile weight when every weigh-in is old', () {
      final ref = ProteinGoalCalc.referenceWeight(
        [_entry(daysAgo(7), 90.0), _entry(daysAgo(30), 95.0)],
        today: today,
        profileWeightKg: 81.0,
      );

      expect(ref.weightKg, 81.0);
      expect(ref.isFromWeightLog, isFalse);
    });

    test('averages the weigh-ins of the 7 days ending today', () {
      final ref = ProteinGoalCalc.referenceWeight(
        [
          _entry(today, 80.0),
          _entry(daysAgo(3), 81.0),
          // Six days ago opens the window; seven days ago is outside it.
          _entry(daysAgo(6), 82.0),
          _entry(daysAgo(7), 99.0),
        ],
        today: today,
        profileWeightKg: 70.0,
      );

      expect(ref.weightKg, closeTo(81.0, 1e-9));
      expect(ref.sampleCount, 3);
      expect(ref.isFromWeightLog, isTrue);
    });

    test('ignores entries dated after today', () {
      final ref = ProteinGoalCalc.referenceWeight(
        [_entry(daysAgo(1), 80.0), _entry(DateTime(2026, 10, 28), 99.0)],
        today: today,
        profileWeightKg: 70.0,
      );

      expect(ref.weightKg, 80.0);
      expect(ref.sampleCount, 1);
    });

    test('counts a weigh-in logged later in the day as today', () {
      final ref = ProteinGoalCalc.referenceWeight(
        [_entry(DateTime(2026, 10, 27, 7, 30), 80.0)],
        today: today,
        profileWeightKg: 70.0,
      );

      expect(ref.weightKg, 80.0);
    });
  });

  group('ProteinGoalCalc.goalGrams', () {
    test('defaults to 1.8 g per kg', () {
      expect(ProteinGoalCalc.goalGrams(100.0), 180);
    });

    test('rounds to the nearest 5 g', () {
      // 1.8 × 82.4 = 148.32
      expect(ProteinGoalCalc.goalGrams(82.4), 150);
      // 1.8 × 80.0 = 144.0
      expect(ProteinGoalCalc.goalGrams(80.0), 145);
      // 1.8 × 79.0 = 142.2
      expect(ProteinGoalCalc.goalGrams(79.0), 140);
    });

    test('a small weight change does not move the goal', () {
      expect(ProteinGoalCalc.goalGrams(82.1), ProteinGoalCalc.goalGrams(82.9));
    });

    test('uses a custom factor', () {
      // 2.2 × 76.4 = 168.08
      expect(ProteinGoalCalc.goalGrams(76.4, gramsPerKg: 2.2), 170);
      // 1.6 × 76.4 = 122.24
      expect(ProteinGoalCalc.goalGrams(76.4, gramsPerKg: 1.6), 120);
    });
  });

  group('MacroCalc carbs/fat split of the kcal left after protein', () {
    test('defaults to the 60:25 ratio', () {
      // 2500 − 150 × 4 = 1900 kcal left.
      expect(
        MacroCalc.getTotalCarbsGoal(2500, 150),
        closeTo(1900 * 0.6 / 0.85 / 4, 1e-9),
      );
      expect(
        MacroCalc.getTotalFatsGoal(2500, 150),
        closeTo(1900 * 0.25 / 0.85 / 9, 1e-9),
      );
    });

    test('keeps the ratio of the stored percentages', () {
      // A stored 50/20/30 split keeps carbs:fat at 50:30.
      expect(
        MacroCalc.carbsShareOfRemainder(userCarbsGoal: 0.5, userFatsGoal: 0.3),
        0.625,
      );
      expect(
        MacroCalc.getTotalCarbsGoal(
          2000,
          100,
          userCarbsGoal: 0.5,
          userFatsGoal: 0.3,
        ),
        closeTo(1600 * 0.625 / 4, 1e-9),
      );
    });

    test('protein, carbs and fat energy add up to the calorie goal', () {
      const goal = 2734.5;
      const protein = 165.0;
      final kcal =
          protein * MacroCalc.proteinKcalPerGram +
          MacroCalc.getTotalCarbsGoal(goal, protein) *
              MacroCalc.carbsKcalPerGram +
          MacroCalc.getTotalFatsGoal(goal, protein) * MacroCalc.fatKcalPerGram;

      expect(kcal, closeTo(goal, 1e-9));
    });

    test('leaves nothing, not a negative goal, when protein exceeds kcal', () {
      expect(MacroCalc.kcalLeftAfterProtein(500, 200), 0);
      expect(MacroCalc.getTotalCarbsGoal(500, 200), 0);
      expect(MacroCalc.getTotalFatsGoal(500, 200), 0);
    });

    test('a zero carbs and fat split falls back to the default ratio', () {
      expect(
        MacroCalc.carbsShareOfRemainder(userCarbsGoal: 0, userFatsGoal: 0),
        closeTo(0.6 / 0.85, 1e-9),
      );
    });

    test('activity energy raises carbs and fat only', () {
      expect(
        MacroCalc.getActivityCarbsIncrease(400) * MacroCalc.carbsKcalPerGram +
            MacroCalc.getActivityFatsIncrease(400) * MacroCalc.fatKcalPerGram,
        closeTo(400, 1e-9),
      );
    });
  });
}
