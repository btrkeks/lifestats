import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';

extension type const FoodKey._(String value) {
  static FoodKey? of(MealEntity meal) {
    final code = meal.code?.trim();
    if (code != null && code.isNotEmpty) {
      return FoodKey._('src:${meal.source.name}:$code');
    }
    final name = normalizeName(meal.name);
    return name == null ? null : FoodKey._('name:$name');
  }

  static FoodKey? parse(String? wire) {
    if (wire == null) return null;
    if (wire.startsWith('name:')) {
      final name = wire.substring(5);
      return name.isNotEmpty && normalizeName(name) == name
          ? FoodKey._(wire)
          : null;
    }
    if (!wire.startsWith('src:')) return null;
    final separator = wire.indexOf(':', 4);
    if (separator < 0) return null;
    final source = wire.substring(4, separator);
    final code = wire.substring(separator + 1);
    if (!MealSourceEntity.values.any((value) => value.name == source) ||
        code.isEmpty ||
        code.trim() != code) {
      return null;
    }
    return FoodKey._(wire);
  }

  static String? normalizeName(String? name) {
    final normalized = name?.trim().toLowerCase().replaceAll(
      RegExp(r'\s+'),
      ' ',
    );
    return normalized == null || normalized.isEmpty ? null : normalized;
  }
}
