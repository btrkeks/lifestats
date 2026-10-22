import 'package:hive_ce_flutter/hive_flutter.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:opennutritracker/core/data/dbo/meal_dbo.dart';

part 'recipe_ingredient_dbo.g.dart';

@HiveType(typeId: 17)
@JsonSerializable()
class RecipeIngredientDBO extends HiveObject {
  @HiveField(0)
  final MealDBO snapshotMeal;

  @HiveField(1)
  final double amount;

  @HiveField(2)
  final String unit;

  // Pre-computed at save time so ComputeRecipeNutritionUseCase doesn't have to
  // re-run unit conversion when displaying or re-aggregating.
  @HiveField(3)
  final double convertedAmountG;

  RecipeIngredientDBO({
    required this.snapshotMeal,
    required this.amount,
    required this.unit,
    required this.convertedAmountG,
  });

  factory RecipeIngredientDBO.fromJson(Map<String, dynamic> json) =>
      _$RecipeIngredientDBOFromJson(json);

  Map<String, dynamic> toJson() => _$RecipeIngredientDBOToJson(this);
}
