import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_type_entity.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_nutriments_entity.dart';

MealEntity food({
  String? code = 'food',
  String? name = 'Food',
  MealSourceEntity source = MealSourceEntity.off,
  double? serving = 250,
  String? servingSize,
  String unit = 'g',
  double? protein = 12,
  double? kcal = 67,
}) => MealEntity(
  code: code,
  name: name,
  url: null,
  mealQuantity: '100',
  mealUnit: unit,
  servingQuantity: serving,
  servingUnit: unit,
  servingSize: servingSize,
  nutriments: MealNutrimentsEntity(
    energyKcal100: kcal,
    proteins100: protein,
    carbohydrates100: null,
    fat100: null,
    sugars100: null,
    saturatedFat100: null,
    fiber100: null,
  ),
  source: source,
);

IntakeEntity intake(
  String id,
  DateTime at, {
  String? code = 'food',
  String? name = 'Food',
  double amount = 250,
  String unit = 'g',
  MealEntity? meal,
}) => IntakeEntity(
  id: id,
  dateTime: at,
  amount: amount,
  unit: unit,
  type: IntakeTypeEntity.snack,
  meal: meal ?? food(code: code, name: name),
);
