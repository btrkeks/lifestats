/// Validates food/meal names entered by the user.
///
/// A name is considered valid when, after trimming surrounding whitespace,
/// it is non-empty and contains at least one alphabetic character.
/// This rejects names that are entirely numeric, made up of only special
/// characters, or blank (see issues #211 and #214).
class FoodNameValidator {
  static final RegExp _letterRegExp = RegExp(r'\p{L}', unicode: true);

  /// Returns `true` when [name] is a valid food name.
  static bool isValid(String? name) {
    if (name == null) return false;
    final trimmed = name.trim();
    if (trimmed.isEmpty) return false;
    return _letterRegExp.hasMatch(trimmed);
  }
}
