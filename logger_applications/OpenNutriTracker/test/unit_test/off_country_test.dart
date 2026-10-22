import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/utils/off_country.dart';

void main() {
  group('OffCountry.fromLocale', () {
    test('maps common locales to their OFF country tag', () {
      expect(OffCountry.fromLocale('en_GB'), 'en:united-kingdom');
      expect(OffCountry.fromLocale('en_US'), 'en:united-states');
      expect(OffCountry.fromLocale('de_DE'), 'en:germany');
      expect(OffCountry.fromLocale('fr_FR'), 'en:france');
      expect(OffCountry.fromLocale('cs_CZ'), 'en:czech-republic');
    });

    test('accepts hyphen separators and trailing encodings', () {
      expect(OffCountry.fromLocale('en-GB'), 'en:united-kingdom');
      expect(OffCountry.fromLocale('en_GB.UTF-8'), 'en:united-kingdom');
      expect(OffCountry.fromLocale('de_AT.utf8'), 'en:austria');
    });

    test('is case-insensitive on the country segment', () {
      expect(OffCountry.fromLocale('en_gb'), 'en:united-kingdom');
    });

    test('returns null when there is no country segment', () {
      expect(OffCountry.fromLocale('en'), isNull);
      expect(OffCountry.fromLocale('C'), isNull);
      expect(OffCountry.fromLocale(''), isNull);
    });

    test('returns null for an unmapped country (graceful no-boost)', () {
      expect(OffCountry.fromLocale('en_ZZ'), isNull);
    });
  });
}
