class KcalGauge {
  final double value;
  final double fraction;
  final bool isOver;

  const KcalGauge(this.value, this.fraction, this.isOver);
}

class KcalGaugeCalc {
  const KcalGaugeCalc._();

  static KcalGauge of({required double kcalLeft, required double kcalGoal}) {
    if (kcalLeft > kcalGoal) return KcalGauge(kcalGoal, 0, false);
    if (kcalLeft < 0) return KcalGauge(kcalLeft.abs(), 1, true);
    return KcalGauge(
      kcalLeft,
      kcalGoal > 0 ? (kcalGoal - kcalLeft) / kcalGoal : 0,
      false,
    );
  }
}
