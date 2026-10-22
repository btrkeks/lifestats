part of 'products_bloc.dart';

abstract class ProductsEvent extends Equatable {
  const ProductsEvent();

  @override
  List<Object?> get props => [];
}

/// Immediate search — fired on submit (enter / search button) and tab change.
/// Bypasses the debounce and always queries the remote source.
class LoadProductsEvent extends ProductsEvent {
  final String searchString;

  const LoadProductsEvent({required this.searchString});

  @override
  List<Object?> get props => [searchString];
}

/// Search-as-you-type — fired on every keystroke and debounced in the bloc.
/// Short queries resolve from local matches only; the remote source is queried
/// once the trimmed query is long enough.
class SearchInputChangedEvent extends ProductsEvent {
  final String searchString;

  const SearchInputChangedEvent({required this.searchString});

  @override
  List<Object?> get props => [searchString];
}

class RefreshProductsEvent extends ProductsEvent {
  const RefreshProductsEvent();

  @override
  List<Object?> get props => [];
}
