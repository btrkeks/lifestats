part of 'recipe_detail_bloc.dart';

sealed class RecipeDetailEvent extends Equatable {
  const RecipeDetailEvent();

  @override
  List<Object?> get props => [];
}

class LoadRecipeEvent extends RecipeDetailEvent {
  final String id;
  const LoadRecipeEvent(this.id);

  @override
  List<Object?> get props => [id];
}

class DeleteRecipeFromDetailEvent extends RecipeDetailEvent {
  const DeleteRecipeFromDetailEvent();
}
