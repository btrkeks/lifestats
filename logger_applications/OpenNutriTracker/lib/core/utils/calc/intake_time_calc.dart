import 'package:opennutritracker/core/utils/calc/day_boundary_calc.dart';

/// The clock time stored on an intake.
///
/// An intake's `dateTime` is the moment it was eaten, and
/// [DayBoundaryCalc] files it under a logical day. A bare midnight is the
/// exception: it is a day label, written by builds that stamped diary
/// entries with the calendar cell and by date-only imports, and it carries
/// no time. Everything here keeps new timestamps out of that shape, so a
/// stored time is always one the user lived or chose.
class IntakeTimeCalc {
  IntakeTimeCalc._();

  /// False for a day label: the day is known, the time is not.
  static bool hasClockTime(DateTime dateTime) =>
      !DayBoundaryCalc.isDayLabel(dateTime);

  /// The label of the logical day [dateTime] is filed under.
  static DateTime logicalDayOf(DateTime dateTime, int? offsetTotalMinutes) =>
      hasClockTime(dateTime)
      ? DayBoundaryCalc.logicalDayOfMinutes(dateTime, offsetTotalMinutes)
      : DateTime(dateTime.year, dateTime.month, dateTime.day);

  /// The timestamp for an intake saved at [now] for [day].
  ///
  /// A clock reading in [day] only means "the day on screen", and screens
  /// capture it when they build, which can be hours before the save; the
  /// intake gets [now]. A day label means the user picked a day in the
  /// diary; the intake gets [now]'s time of day on that logical day, which
  /// is [now] itself when the picked day is today.
  static DateTime stampFor(
    DateTime day,
    DateTime now,
    int? offsetTotalMinutes,
  ) {
    if (hasClockTime(day)) return now;
    return onLogicalDay(
      DateTime(day.year, day.month, day.day),
      now,
      offsetTotalMinutes,
    );
  }

  /// [dateTime] moved to [hour]:[minute] on the logical day it is filed
  /// under, so editing the time never moves an intake to another day.
  static DateTime withTimeOfDay(
    DateTime dateTime,
    int hour,
    int minute,
    int? offsetTotalMinutes,
  ) => onLogicalDay(
    logicalDayOf(dateTime, offsetTotalMinutes),
    DateTime(2000, 1, 1, hour, minute),
    offsetTotalMinutes,
  );

  /// [timeOfDay]'s clock time on the logical day labelled [dayLabel].
  ///
  /// With a day start of 04:00, 02:30 on the 20th's logical day is the
  /// 21st at 02:30. A result of exactly midnight is pushed one microsecond
  /// later so it is not mistaken for a day label.
  static DateTime onLogicalDay(
    DateTime dayLabel,
    DateTime timeOfDay,
    int? offsetTotalMinutes,
  ) {
    DateTime on(int dayShift) {
      final moment = DateTime(
        dayLabel.year,
        dayLabel.month,
        dayLabel.day + dayShift,
        timeOfDay.hour,
        timeOfDay.minute,
        timeOfDay.second,
        timeOfDay.millisecond,
        timeOfDay.microsecond,
      );
      return DayBoundaryCalc.isDayLabel(moment)
          ? moment.add(const Duration(microseconds: 1))
          : moment;
    }

    final sameDate = on(0);
    return DayBoundaryCalc.isMomentInLogicalDayMinutes(
          dayLabel,
          sameDate,
          offsetTotalMinutes,
        )
        ? sameDate
        : on(1);
  }
}
