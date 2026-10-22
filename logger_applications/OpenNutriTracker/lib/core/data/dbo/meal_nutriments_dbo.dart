import 'package:hive_ce_flutter/hive_flutter.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_nutriments_entity.dart';

part 'meal_nutriments_dbo.g.dart';

@HiveType(typeId: 3)
@JsonSerializable()
class MealNutrimentsDBO extends HiveObject {
  @HiveField(0)
  final double? energyKcal100;
  @HiveField(1)
  final double? carbohydrates100;
  @HiveField(2)
  final double? fat100;
  @HiveField(3)
  final double? proteins100;
  @HiveField(4)
  final double? sugars100;
  @HiveField(5)
  final double? saturatedFat100;
  @HiveField(6)
  final double? fiber100;
  // #237: Extended lipid profile
  @HiveField(7)
  final double? monounsaturatedFat100;
  @HiveField(8)
  final double? polyunsaturatedFat100;
  @HiveField(9)
  final double? transFat100;
  @HiveField(10)
  final double? cholesterol100;
  // #237: Minerals (mg per 100g)
  @HiveField(11)
  final double? sodium100;
  @HiveField(12)
  final double? potassium100;
  @HiveField(13)
  final double? magnesium100;
  @HiveField(14)
  final double? calcium100;
  @HiveField(15)
  final double? iron100;
  @HiveField(16)
  final double? zinc100;
  @HiveField(17)
  final double? phosphorus100;
  // #237: Vitamins
  @HiveField(18)
  final double? vitaminA100; // µg RAE
  @HiveField(19)
  final double? vitaminC100; // mg
  @HiveField(20)
  final double? vitaminD100; // µg
  @HiveField(21)
  final double? vitaminB6100; // mg
  @HiveField(22)
  final double? vitaminB12100; // µg
  @HiveField(23)
  final double? niacin100; // mg (B3)

  MealNutrimentsDBO({
    required this.energyKcal100,
    required this.carbohydrates100,
    required this.fat100,
    required this.proteins100,
    required this.sugars100,
    required this.saturatedFat100,
    required this.fiber100,
    this.monounsaturatedFat100,
    this.polyunsaturatedFat100,
    this.transFat100,
    this.cholesterol100,
    this.sodium100,
    this.potassium100,
    this.magnesium100,
    this.calcium100,
    this.iron100,
    this.zinc100,
    this.phosphorus100,
    this.vitaminA100,
    this.vitaminC100,
    this.vitaminD100,
    this.vitaminB6100,
    this.vitaminB12100,
    this.niacin100,
  });

  factory MealNutrimentsDBO.fromProductNutrimentsEntity(
    MealNutrimentsEntity nutriments,
  ) {
    return MealNutrimentsDBO(
      energyKcal100: nutriments.energyKcal100,
      carbohydrates100: nutriments.carbohydrates100,
      fat100: nutriments.fat100,
      proteins100: nutriments.proteins100,
      sugars100: nutriments.sugars100,
      saturatedFat100: nutriments.saturatedFat100,
      fiber100: nutriments.fiber100,
      monounsaturatedFat100: nutriments.monounsaturatedFat100,
      polyunsaturatedFat100: nutriments.polyunsaturatedFat100,
      transFat100: nutriments.transFat100,
      cholesterol100: nutriments.cholesterol100,
      sodium100: nutriments.sodium100,
      potassium100: nutriments.potassium100,
      magnesium100: nutriments.magnesium100,
      calcium100: nutriments.calcium100,
      iron100: nutriments.iron100,
      zinc100: nutriments.zinc100,
      phosphorus100: nutriments.phosphorus100,
      vitaminA100: nutriments.vitaminA100,
      vitaminC100: nutriments.vitaminC100,
      vitaminD100: nutriments.vitaminD100,
      vitaminB6100: nutriments.vitaminB6100,
      vitaminB12100: nutriments.vitaminB12100,
      niacin100: nutriments.niacin100,
    );
  }

  factory MealNutrimentsDBO.fromJson(Map<String, dynamic> json) =>
      _$MealNutrimentsDBOFromJson(json);

  Map<String, dynamic> toJson() => _$MealNutrimentsDBOToJson(this);
}
