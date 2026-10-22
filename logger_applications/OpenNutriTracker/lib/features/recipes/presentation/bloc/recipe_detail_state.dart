part of 'recipe_detail_bloc.dart';

sealed class RecipeDetailState extends Equatable {
  const RecipeDetailState();

  @override
  List<Object?> get props => [];
}

class RecipeDetailInitial extends RecipeDetailState {
  const RecipeDetailInitial();
}

class RecipeDetailLoaded extends RecipeDetailState {
  final RecipeEntity recipe;
  const RecipeDetailLoaded(this.recipe);

  @override
  List<Object?> get props => [recipe];
}

class RecipeDetailMissing extends RecipeDetailState {
  const RecipeDetailMissing();
}

class RecipeDetailDeleted extends RecipeDetailState {
  const RecipeDetailDeleted();
}
