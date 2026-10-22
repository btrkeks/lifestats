import 'package:equatable/equatable.dart';
import 'package:opennutritracker/core/data/dbo/meal_dbo.dart';
import 'package:opennutritracker/core/data/dbo/recipe_ingredient_dbo.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';

class RecipeIngredientEntity extends Equatable {
  final MealEntity snapshotMeal;
  final double amount;
  final String unit;
  final double convertedAmountG;

  const RecipeIngredientEntity({
    required this.snapshotMeal,
    required this.amount,
    required this.unit,
    required this.convertedAmountG,
  });

  factory RecipeIngredientEntity.fromDBO(RecipeIngredientDBO dbo) {
    return RecipeIngredientEntity(
      snapshotMeal: MealEntity.fromMealDBO(dbo.snapshotMeal),
      amount: dbo.amount,
      unit: dbo.unit,
      convertedAmountG: dbo.convertedAmountG,
    );
  }

  RecipeIngredientDBO toDBO() {
    return RecipeIngredientDBO(
      snapshotMeal: MealDBO.fromMealEntity(snapshotMeal),
      amount: amount,
      unit: unit,
      convertedAmountG: convertedAmountG,
    );
  }

  RecipeIngredientEntity copyWith({
    MealEntity? snapshotMeal,
    double? amount,
    String? unit,
    double? convertedAmountG,
  }) {
    return RecipeIngredientEntity(
      snapshotMeal: snapshotMeal ?? this.snapshotMeal,
      amount: amount ?? this.amount,
      unit: unit ?? this.unit,
      convertedAmountG: convertedAmountG ?? this.convertedAmountG,
    );
  }

  @override
  List<Object?> get props => [snapshotMeal, amount, unit, convertedAmountG];
}
