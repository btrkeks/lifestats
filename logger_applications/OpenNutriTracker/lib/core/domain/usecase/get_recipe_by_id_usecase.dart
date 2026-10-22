import 'package:opennutritracker/core/data/repository/recipe_repository.dart';
import 'package:opennutritracker/core/domain/entity/recipe_entity.dart';

class GetRecipeByIdUseCase {
  final RecipeRepository _repository;

  GetRecipeByIdUseCase(this._repository);

  RecipeEntity? getById(String id) => _repository.getRecipeById(id);
}
