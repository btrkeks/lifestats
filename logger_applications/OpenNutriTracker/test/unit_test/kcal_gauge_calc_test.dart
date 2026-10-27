import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/utils/calc/kcal_gauge_calc.dart';

void main() {
  for (final (left, goal, value, fraction, over) in [
    (2500.0, 2400.0, 2400.0, 0.0, false),
    (-150.0, 2400.0, 150.0, 1.0, true),
    (1800.0, 2400.0, 1800.0, 0.25, false),
    (0.0, 2400.0, 0.0, 1.0, false),
    (0.0, 0.0, 0.0, 0.0, false),
  ]) {
    test(
      '$left left of $goal: value $value, fraction $fraction, over $over',
      () {
        final gauge = KcalGaugeCalc.of(kcalLeft: left, kcalGoal: goal);
        expect(gauge.value, value);
        expect(gauge.fraction, fraction);
        expect(gauge.isOver, over);
      },
    );
  }
}
