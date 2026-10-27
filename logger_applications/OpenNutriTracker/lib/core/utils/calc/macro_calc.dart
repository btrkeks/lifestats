import 'dart:math';

class MacroCalc {
  /// Information provided by
  /// 'OBESITY: PREVENTING AND MANAGING
  /// THE GLOBAL EPIDEMIC' by WHO page 104
  /// ISBN 92 4 120894 5
  /// ISSN 0512-3054
  ///
  /// Public (rather than private) so the calorie-goal transparency screen
  /// can display the exact densities and default split it computes with.
  static const carbsKcalPerGram = 4.0;
  static const fatKcalPerGram = 9.0;
  static const proteinKcalPerGram = 4.0;

  /// Default carbs and fat percentages. Protein is set in grams from body
  /// weight (see `ProteinGoalCalc`), so only the ratio between these two
  /// matters: they split whatever energy protein leaves over.
  static const defaultCarbsPercentageGoal = 0.6;
  static const defaultFatsPercentageGoal = 0.25;

  /// Carbs' share of the energy left after protein, from the ratio of the
  /// stored carbs and fat percentages (60:25 by default, about 71 %).
  static double carbsShareOfRemainder({
    double? userCarbsGoal,
    double? userFatsGoal,
  }) {
    final carbs = userCarbsGoal ?? defaultCarbsPercentageGoal;
    final fats = userFatsGoal ?? defaultFatsPercentageGoal;
    if (carbs + fats <= 0) {
      return defaultCarbsPercentageGoal /
          (defaultCarbsPercentageGoal + defaultFatsPercentageGoal);
    }
    return carbs / (carbs + fats);
  }

  /// Energy left for carbs and fat once [proteinGrams] are covered. Never
  /// negative: a protein goal above the calorie goal leaves zero, not debt.
  static double kcalLeftAfterProtein(
    double totalCalorieGoal,
    double proteinGrams,
  ) => max(0, totalCalorieGoal - proteinGrams * proteinKcalPerGram);

  /// Carbs goal in grams: carbs' share of the kcal left after protein.
  static double getTotalCarbsGoal(
    double totalCalorieGoal,
    double proteinGrams, {
    double? userCarbsGoal,
    double? userFatsGoal,
  }) =>
      kcalLeftAfterProtein(totalCalorieGoal, proteinGrams) *
      carbsShareOfRemainder(
        userCarbsGoal: userCarbsGoal,
        userFatsGoal: userFatsGoal,
      ) /
      carbsKcalPerGram;

  /// Fat goal in grams: fat's share of the kcal left after protein.
  static double getTotalFatsGoal(
    double totalCalorieGoal,
    double proteinGrams, {
    double? userCarbsGoal,
    double? userFatsGoal,
  }) =>
      kcalLeftAfterProtein(totalCalorieGoal, proteinGrams) *
      (1 -
          carbsShareOfRemainder(
            userCarbsGoal: userCarbsGoal,
            userFatsGoal: userFatsGoal,
          )) /
      fatKcalPerGram;

  /// Carbs added to a day's goal by [burnedKcal] of logged activity. Burned
  /// energy goes to carbs and fat only; the protein goal follows body
  /// weight, not energy.
  static double getActivityCarbsIncrease(double burnedKcal) =>
      getTotalCarbsGoal(burnedKcal, 0);

  /// Fat added to a day's goal by [burnedKcal] of logged activity.
  static double getActivityFatsIncrease(double burnedKcal) =>
      getTotalFatsGoal(burnedKcal, 0);
}
