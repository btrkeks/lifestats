import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_amount.dart';
import 'package:opennutritracker/features/meal_detail/util/meal_quantity_converter.dart';
import 'quick_log_test_data.dart';

void main() {
  final meal = food();
  test('base units and ambiguous g/ml remain base quantities', () {
    for (final unit in ['g', 'ml', 'g/ml']) {
      final display = displayQuantityOf(
        baseAmount: 180,
        unit: unit,
        meal: meal,
      );
      expect(display.quantity, 180);
      expect(display.unit, unit == 'g/ml' ? 'g' : unit);
    }
  });
  test('oz and fl.oz reverse the actual converter', () {
    for (final unit in ['oz', 'fl.oz']) {
      final base = convertQuantityToBaseUnit(4, unit, meal);
      final display = displayQuantityOf(
        baseAmount: base,
        unit: unit,
        meal: meal,
      );
      expect(display.quantity, closeTo(4, 1e-10));
      expect(display.unit, unit);
    }
  });
  test('clean half servings and textual serving weights are recovered', () {
    for (final meal in [
      food(),
      food(serving: null, servingSize: '1 tub (250 g)'),
    ]) {
      final display = displayQuantityOf(
        baseAmount: 375,
        unit: 'serving',
        meal: meal,
      );
      expect(display.quantity, 1.5);
      expect(display.unit, 'serving');
    }
  });
  test('ragged or unavailable servings fall back to grams or ml', () {
    final ragged = displayQuantityOf(
      baseAmount: 180,
      unit: 'serving',
      meal: meal,
    );
    expect((ragged.quantity, ragged.unit), (180, 'g'));
    final absent = displayQuantityOf(
      baseAmount: 180,
      unit: 'serving',
      meal: food(serving: null, unit: 'ml'),
    );
    expect((absent.quantity, absent.unit), (180, 'ml'));
  });
  test('picker units follow food units and valid serving availability', () {
    expect(pickerUnitsFor(meal, imperial: true), ['g', 'serving', 'oz']);
    expect(pickerUnitsFor(food(unit: 'ml'), imperial: true), [
      'ml',
      'serving',
      'fl.oz',
    ]);
    for (final serving in [null, 0.0, -1.0, double.nan, double.infinity]) {
      expect(pickerUnitsFor(food(serving: serving), imperial: false), ['g']);
    }
  });
  test('basePerUnit equals the converter for every supported unit', () {
    for (final meal in [food(), food(unit: 'ml'), food(serving: null)]) {
      for (final unit in ['g', 'ml', 'g/ml', 'serving', 'oz', 'fl.oz']) {
        expect(
          basePerUnit(unit, meal),
          convertQuantityToBaseUnit(1, unit, meal),
        );
      }
    }
  });
}
