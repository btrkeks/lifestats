import 'package:opennutritracker/core/data/repository/recipe_repository.dart';
import 'package:opennutritracker/core/domain/entity/recipe_entity.dart';
import 'package:opennutritracker/core/domain/usecase/compute_recipe_nutrition_usecase.dart';

class SaveRecipeUseCase {
  final RecipeRepository _repository;
  final ComputeRecipeNutritionUseCase _computeUseCase;

  SaveRecipeUseCase(this._repository, this._computeUseCase);

  // Always re-aggregates from the current ingredient list before persisting.
  // Past IntakeDBO rows already snapshot the meal, so this never changes
  // historical entries — it only affects future logs of the recipe.
  Future<RecipeEntity> save(
    RecipeEntity recipe, {
    bool totalWeightOverridden = false,
  }) async {
    final result = _computeUseCase.compute(
      recipe.ingredients,
      totalWeightOverride: totalWeightOverridden ? recipe.totalWeightG : null,
    );

    final updated = recipe.copyWith(
      totalWeightG: result.totalWeightG,
      aggregatedNutrimentsPer100: result.perHundredG,
      updatedAt: DateTime.now(),
    );

    await _repository.saveRecipe(updated);
    return updated;
  }
}
