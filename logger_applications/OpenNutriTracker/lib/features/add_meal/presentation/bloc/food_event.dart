part of 'food_bloc.dart';

abstract class FoodEvent extends Equatable {
  const FoodEvent();

  @override
  List<Object?> get props => [];
}

/// Immediate search — fired on submit (enter / search button) and tab change.
/// Bypasses the debounce and always queries the remote source.
class LoadFoodEvent extends FoodEvent {
  final String searchString;

  const LoadFoodEvent({required this.searchString});

  @override
  List<Object?> get props => [searchString];
}

/// Search-as-you-type — fired on every keystroke and debounced in the bloc.
class SearchFoodInputChangedEvent extends FoodEvent {
  final String searchString;

  const SearchFoodInputChangedEvent({required this.searchString});

  @override
  List<Object?> get props => [searchString];
}

class RefreshFoodEvent extends FoodEvent {
  const RefreshFoodEvent();

  @override
  List<Object?> get props => [];
}
