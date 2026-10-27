import 'package:opennutritracker/core/domain/entity/weight_log_entity.dart';
import 'package:opennutritracker/core/utils/calc/weight_moving_average.dart';

/// The body weight a protein goal is scaled from.
class ProteinReferenceWeight {
  final double weightKg;

  /// Weigh-ins averaged into [weightKg]. `0` means the weight log held no
  /// reading in the window and the profile weight stood in.
  final int sampleCount;

  const ProteinReferenceWeight({
    required this.weightKg,
    required this.sampleCount,
  });

  bool get isFromWeightLog => sampleCount > 0;
}

/// Daily protein goal as a multiple of total body weight.
///
/// Morton et al. (2018, Br J Sports Med 52:376–384) found gains in lean
/// mass from extra protein plateau around 1.6 g per kg of body weight per
/// day, with the upper bound of the confidence interval near 2.2 g/kg.
/// [defaultGramsPerKg] sits between the two. The meta-analysis scaled by
/// total body weight, so that is the reference here, not lean mass.
class ProteinGoalCalc {
  static const defaultGramsPerKg = 1.8;
  static const minGramsPerKg = 0.8;
  static const maxGramsPerKg = 3.0;

  /// Calendar days averaged into the reference weight, today included.
  static const averageWindowDays = 7;

  /// The goal is rounded to this step so day-to-day weight noise doesn't
  /// move the target by a gram or two every morning.
  static const roundingStepGrams = 5.0;

  /// Mean of the weigh-ins dated within the [averageWindowDays] ending on
  /// [today], falling back to [profileWeightKg] when there are none.
  static ProteinReferenceWeight referenceWeight(
    Iterable<WeightLogEntity> entries, {
    required DateTime today,
    required double profileWeightKg,
  }) {
    final windowStart = DateTime(
      today.year,
      today.month,
      today.day - (averageWindowDays - 1),
    );
    final windowEnd = DateTime(today.year, today.month, today.day + 1);
    final recent = entries.where(
      (e) => !e.date.isBefore(windowStart) && e.date.isBefore(windowEnd),
    );
    // Every entry left lies inside one window, so the last point of the
    // moving average is the mean of all of them.
    final points = const WeightMovingAverage(
      windowDays: averageWindowDays,
    ).compute(recent, minSamples: 1);
    if (points.isEmpty) {
      return ProteinReferenceWeight(weightKg: profileWeightKg, sampleCount: 0);
    }
    return ProteinReferenceWeight(
      weightKg: points.last.weightKg,
      sampleCount: points.last.sampleCount,
    );
  }

  /// Protein goal in grams: [gramsPerKg] × [referenceWeightKg], rounded to
  /// the nearest [roundingStepGrams].
  static double goalGrams(double referenceWeightKg, {double? gramsPerKg}) {
    final raw = referenceWeightKg * (gramsPerKg ?? defaultGramsPerKg);
    return (raw / roundingStepGrams).round() * roundingStepGrams;
  }
}
