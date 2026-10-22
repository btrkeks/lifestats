import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/utils/food_name_validator.dart';

void main() {
  group('FoodNameValidator', () {
    test('rejects null name', () {
      expect(FoodNameValidator.isValid(null), isFalse);
    });

    test('rejects empty string (#214)', () {
      expect(FoodNameValidator.isValid(''), isFalse);
    });

    test('rejects whitespace-only string (#214)', () {
      expect(FoodNameValidator.isValid('   '), isFalse);
      expect(FoodNameValidator.isValid('\t\n'), isFalse);
    });

    test('rejects numeric-only name (#211)', () {
      expect(FoodNameValidator.isValid('123456'), isFalse);
    });

    test('rejects special-character-only name (#211)', () {
      expect(FoodNameValidator.isValid('@#\$%^'), isFalse);
      expect(FoodNameValidator.isValid('---'), isFalse);
    });

    test('accepts simple alphabetic name', () {
      expect(FoodNameValidator.isValid('Apple'), isTrue);
    });

    test('accepts name mixing letters with numbers and symbols', () {
      expect(FoodNameValidator.isValid('Coke 500ml'), isTrue);
      expect(FoodNameValidator.isValid('Mac & Cheese'), isTrue);
    });

    test('accepts name with leading or trailing whitespace', () {
      expect(FoodNameValidator.isValid('  Apple  '), isTrue);
    });

    test('accepts non-ASCII alphabetic characters', () {
      expect(FoodNameValidator.isValid('Maçã'), isTrue); // Portuguese
      expect(FoodNameValidator.isValid('Käse'), isTrue); // German
      expect(FoodNameValidator.isValid('日本食'), isTrue); // Japanese
    });
  });
}
