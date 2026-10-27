import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/utils/calc/day_boundary_calc.dart';
import 'package:opennutritracker/features/quick_log/domain/food_key.dart';

class Suggestion {
  final FoodKey key;
  final IntakeEntity latest;
  final List<IntakeEntity> recentAmounts;

  const Suggestion({
    required this.key,
    required this.latest,
    required this.recentAmounts,
  });
}

/// D-7…D-1 counts freeze the head; first logs today append to its tail.
List<Suggestion> rankSuggestions({
  required Iterable<IntakeEntity> intakes,
  required DateTime today,
  required int dayStartOffsetMinutes,
  int windowDays = 7,
}) {
  final days = List.generate(
    windowDays + 1,
    (i) => DateTime(today.year, today.month, today.day - i),
  );
  final groups = <FoodKey, _History>{};
  for (final intake in intakes) {
    final key = FoodKey.of(intake.meal);
    if (key == null) continue;
    // Midnight imports are day labels, just as they are on Home.
    final index = days.indexWhere(
      (day) => DayBoundaryCalc.isMomentInLogicalDayMinutes(
        day,
        intake.dateTime,
        dayStartOffsetMinutes,
      ),
    );
    if (index < 0) continue;
    final history = groups.putIfAbsent(key, () => _History(key));
    history.intakes.add(intake);
    if (index == 0) {
      if (history.firstToday == null ||
          intake.dateTime.isBefore(history.firstToday!)) {
        history.firstToday = intake.dateTime;
      }
    } else {
      history.count++;
      if (history.latestBeforeToday == null ||
          intake.dateTime.isAfter(history.latestBeforeToday!)) {
        history.latestBeforeToday = intake.dateTime;
      }
    }
  }
  final histories = groups.values.toList()
    ..sort((a, b) {
      if (a.count != b.count) return b.count.compareTo(a.count);
      final byDate = a.count > 0
          ? b.latestBeforeToday!.compareTo(a.latestBeforeToday!)
          : a.firstToday!.compareTo(b.firstToday!);
      return byDate != 0 ? byDate : a.key.value.compareTo(b.key.value);
    });
  return histories.map((history) {
    history.intakes.sort((a, b) {
      final byDate = b.dateTime.compareTo(a.dateTime);
      return byDate != 0 ? byDate : a.id.compareTo(b.id);
    });
    final seen = <(double, String)>{};
    final recent = history.intakes
        .where((intake) => seen.add((intake.amount, intake.unit)))
        .take(4)
        .toList();
    return Suggestion(
      key: history.key,
      latest: history.intakes.first,
      recentAmounts: recent,
    );
  }).toList();
}

class _History {
  final FoodKey key;
  final List<IntakeEntity> intakes = [];
  int count = 0;
  DateTime? latestBeforeToday;
  DateTime? firstToday;

  _History(this.key);
}
