import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';
import 'package:opennutritracker/features/quick_log/domain/food_key.dart';
import 'quick_log_test_data.dart';

void main() {
  test('source separates equal codes and codes are trimmed', () {
    expect(FoodKey.of(food(code: ' 42 '))?.value, 'src:off:42');
    expect(
      FoodKey.of(food(code: '42', source: MealSourceEntity.custom))?.value,
      'src:custom:42',
    );
  });
  test('null, empty and blank codes use the normalized name', () {
    for (final code in [null, '', '  ']) {
      expect(
        FoodKey.of(food(code: code, name: '  Protein \t SHAKE  '))?.value,
        'name:protein shake',
      );
    }
  });
  test('no identity when both code and name are blank', () {
    expect(FoodKey.of(food(code: '', name: '  ')), isNull);
    expect(FoodKey.of(food(code: null, name: null)), isNull);
  });
  test('wire keys round trip including colons in codes', () {
    for (final meal in [
      food(code: 'namespace:42'),
      food(code: '', name: 'Milk'),
    ]) {
      final key = FoodKey.of(meal)!;
      expect(FoodKey.parse(key.value), key);
    }
  });
  test('malformed wire keys are rejected', () {
    for (final wire in [
      null,
      '',
      'milk',
      'name:',
      'name: Milk ',
      'src:off:',
      'src:fake:123',
      'src:off: 123',
      'src:off',
    ]) {
      expect(FoodKey.parse(wire), isNull, reason: '$wire');
    }
  });
}
