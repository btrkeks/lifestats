import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/features/profile/presentation/utils/profile_display_format.dart';

void main() {
  group('formatProfileWeight', () {
    test('shows whole numbers without decimal suffix', () {
      expect(formatProfileWeight(80.0), '80');
    });

    test('preserves one decimal place for fractional values', () {
      expect(formatProfileWeight(80.5), '80.5');
    });

    test('normalizes floating-point noise around whole values', () {
      expect(formatProfileWeight(154.0000001), '154');
    });

    test('rounds values to one decimal place', () {
      expect(formatProfileWeight(154.25), '154.3');
    });
  });
}
