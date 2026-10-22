part of 'recipes_bloc.dart';

sealed class RecipesState extends Equatable {
  const RecipesState();

  @override
  List<Object?> get props => [];
}

class RecipesInitial extends RecipesState {
  const RecipesInitial();
}

class RecipesLoading extends RecipesState {
  const RecipesLoading();
}

class RecipesLoaded extends RecipesState {
  final List<RecipeEntity> recipes;
  const RecipesLoaded(this.recipes);

  @override
  List<Object?> get props => [recipes];
}

class RecipesFailed extends RecipesState {
  final String message;
  const RecipesFailed(this.message);

  @override
  List<Object?> get props => [message];
}
