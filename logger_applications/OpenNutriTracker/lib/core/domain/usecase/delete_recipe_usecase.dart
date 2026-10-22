import 'package:opennutritracker/core/data/repository/recipe_repository.dart';

// Non-cascading: past IntakeDBO entries that referenced this recipe are
// retained because they hold a frozen snapshot of the meal at log time.
// Their kcal/macros stay accurate; deleting "I ate vanilla cake on Tuesday"
// would feel destructive for an artifact the user spent time creating.
class DeleteRecipeUseCase {
  final RecipeRepository _repository;

  DeleteRecipeUseCase(this._repository);

  Future<void> delete(String id) => _repository.deleteRecipe(id);
}
