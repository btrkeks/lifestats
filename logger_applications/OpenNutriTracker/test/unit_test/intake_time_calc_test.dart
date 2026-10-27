import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/utils/calc/day_boundary_calc.dart';
import 'package:opennutritracker/core/utils/calc/intake_time_calc.dart';

void main() {
  const fourAm = 4 * 60;

  group('hasClockTime', () {
    test('a bare midnight is a day label, local or UTC', () {
      expect(IntakeTimeCalc.hasClockTime(DateTime(2026, 10, 20)), isFalse);
      expect(IntakeTimeCalc.hasClockTime(DateTime.utc(2026, 10, 20)), isFalse);
    });

    test('anything else is a clock reading', () {
      expect(
        IntakeTimeCalc.hasClockTime(DateTime(2026, 10, 20, 0, 0, 0, 0, 1)),
        isTrue,
      );
      expect(
        IntakeTimeCalc.hasClockTime(DateTime(2026, 10, 20, 8, 12)),
        isTrue,
      );
    });
  });

  group('stampFor', () {
    final now = DateTime(2026, 10, 27, 8, 12, 30, 5);

    test('a clock-reading day means today and gets the save time', () {
      // The home screen hands over the moment it was built.
      final builtAt = DateTime(2026, 10, 27, 6, 0);
      expect(IntakeTimeCalc.stampFor(builtAt, now, 0), now);
    });

    test('a past diary day gets the current clock time on that day', () {
      expect(
        IntakeTimeCalc.stampFor(DateTime.utc(2026, 10, 20), now, 0),
        DateTime(2026, 10, 20, 8, 12, 30, 5),
      );
    });

    test("today's diary label gets exactly the save time", () {
      expect(IntakeTimeCalc.stampFor(DateTime.utc(2026, 10, 27), now, 0), now);
    });

    test('before the day start, the time lands on the next calendar date', () {
      final night = DateTime(2026, 10, 28, 2, 30);
      final stamped = IntakeTimeCalc.stampFor(
        DateTime.utc(2026, 10, 20),
        night,
        fourAm,
      );
      expect(stamped, DateTime(2026, 10, 21, 2, 30));
      expect(
        DayBoundaryCalc.isMomentInLogicalDayMinutes(
          DateTime(2026, 10, 20),
          stamped,
          fourAm,
        ),
        isTrue,
      );
      // And on today's logical day it is the save time itself.
      expect(
        IntakeTimeCalc.stampFor(DateTime.utc(2026, 10, 27), night, fourAm),
        night,
      );
    });
  });

  group('withTimeOfDay', () {
    test('keeps the date and replaces the time', () {
      expect(
        IntakeTimeCalc.withTimeOfDay(
          DateTime(2026, 10, 27, 8, 12, 30),
          7,
          45,
          0,
        ),
        DateTime(2026, 10, 27, 7, 45),
      );
    });

    test('gives a day label a time on its own day', () {
      expect(
        IntakeTimeCalc.withTimeOfDay(DateTime.utc(2026, 10, 20), 13, 40, 0),
        DateTime(2026, 10, 20, 13, 40),
      );
    });

    test('never moves an intake to another logical day', () {
      // 23:30 on the 20th, moved to 01:15 with a 04:00 day start: still the
      // 20th's logical day, so the 21st's calendar date.
      final moved = IntakeTimeCalc.withTimeOfDay(
        DateTime(2026, 10, 20, 23, 30),
        1,
        15,
        fourAm,
      );
      expect(moved, DateTime(2026, 10, 21, 1, 15));
      expect(
        IntakeTimeCalc.logicalDayOf(moved, fourAm),
        DateTime(2026, 10, 20),
      );
    });

    test('a picked midnight stays a clock reading on the same day', () {
      for (final offset in [0, fourAm]) {
        final moved = IntakeTimeCalc.withTimeOfDay(
          DateTime(2026, 10, 20, 12),
          0,
          0,
          offset,
        );
        expect(IntakeTimeCalc.hasClockTime(moved), isTrue);
        expect(moved.hour, 0);
        expect(moved.minute, 0);
        expect(
          IntakeTimeCalc.logicalDayOf(moved, offset),
          DateTime(2026, 10, 20),
        );
      }
    });
  });
}
