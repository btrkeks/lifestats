import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';
import 'package:opennutritracker/features/meal_detail/util/meal_quantity_converter.dart';

class DisplayQuantity {
  final double quantity;
  final String unit;

  const DisplayQuantity(this.quantity, this.unit);
}

DisplayQuantity displayQuantityOf({
  required double baseAmount,
  required String unit,
  required MealEntity meal,
}) {
  if (unit == 'g' || unit == 'ml') return DisplayQuantity(baseAmount, unit);
  if (unit == 'g/ml') return DisplayQuantity(baseAmount, 'g');
  if (unit == 'oz' || unit == 'fl.oz') {
    return DisplayQuantity(baseAmount / basePerUnit(unit, meal), unit);
  }
  if (unit == 'serving' && _hasServing(meal)) {
    final quantity = baseAmount / basePerUnit(unit, meal);
    final half = (quantity * 2).round() / 2;
    if (half > 0 && (quantity - half).abs() <= half * 0.01) {
      return DisplayQuantity(quantity, unit);
    }
  }
  return DisplayQuantity(baseAmount, meal.isLiquid ? 'ml' : 'g');
}

double basePerUnit(String unit, MealEntity meal) =>
    convertQuantityToBaseUnit(1, unit, meal);

List<String> pickerUnitsFor(MealEntity meal, {required bool imperial}) => [
  meal.isLiquid ? 'ml' : 'g',
  if (_hasServing(meal)) 'serving',
  if (imperial) meal.isLiquid ? 'fl.oz' : 'oz',
];

bool _hasServing(MealEntity meal) {
  final serving = meal.scalableServingQuantity;
  return serving != null && serving.isFinite && serving > 0;
}
