import 'package:equatable/equatable.dart';
import 'package:logging/logging.dart';
import 'package:opennutritracker/core/data/data_source/lifestats/lifestats_client.dart';
import 'package:opennutritracker/core/data/repository/config_repository.dart';
import 'package:opennutritracker/core/data/repository/intake_repository.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/utils/calc/intake_time_calc.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_nutriments_entity.dart';

/// One intake in a Lifestats TimedAmounts list.
class LifestatsTimedAmount extends Equatable {
  /// Local wall-clock `HH:MM:SS`, or null when the intake only knows its
  /// day.
  final String? time;
  final int amountMg;

  const LifestatsTimedAmount(this.time, this.amountMg);

  Map<String, Object?> toJson() => {'time': time, 'amount_mg': amountMg};

  @override
  List<Object?> get props => [time, amountMg];
}

/// The complete caffeine and nicotine lists of one logical day, the body of
/// `PUT /integrations/opennutritracker/days/{day}`.
class LifestatsIntakeDay extends Equatable {
  /// The logical day's label (local midnight).
  final DateTime day;
  final List<LifestatsTimedAmount> caffeine;
  final List<LifestatsTimedAmount> nicotine;

  const LifestatsIntakeDay(this.day, this.caffeine, this.nicotine);

  String get path =>
      '/integrations/opennutritracker/days/'
      '${day.year.toString().padLeft(4, '0')}-${_twoDigits(day.month)}-'
      '${_twoDigits(day.day)}';

  Map<String, Object> toJson() => {
    'caffeine': [for (final entry in caffeine) entry.toJson()],
    'nicotine': [for (final entry in nicotine) entry.toJson()],
  };

  @override
  List<Object?> get props => [day, caffeine, nicotine];
}

/// Sends each day's caffeine and nicotine intakes to Lifestats.
///
/// Every sync replaces the last [windowDays] logical days, today included,
/// one request per day, oldest first. A day without either nutrient goes
/// out as two empty lists: Lifestats reads that as a confirmed zero, and a
/// day it never received as a missed goal. Sending the whole window each
/// time carries edits and deletes without tracking which days changed;
/// older days keep what was last sent.
///
/// The first failed request stops the sync. Its status is kept apart from
/// the weight sync's, so one succeeding does not hide the other failing.
class SyncLifestatsIntakeUsecase {
  static const windowDays = 14;

  /// The shortest gap between two background syncs, as for the weight sync.
  static const minimumInterval = Duration(minutes: 5);

  /// The clock "today" and the sync timestamps are read from. Replaced in
  /// tests.
  static DateTime Function() clock = DateTime.now;

  final _log = Logger('SyncLifestatsIntakeUsecase');
  final LifestatsClient _client;
  final ConfigRepository _configRepository;
  final IntakeRepository _intakeRepository;

  Future<int>? _inFlight;
  DateTime? _lastBackgroundAttempt;

  SyncLifestatsIntakeUsecase(
    this._client,
    this._configRepository,
    this._intakeRepository,
  );

  /// Background entry point for app start and resume: syncs unless sync is
  /// off or one ran within [minimumInterval], and never throws. A failure
  /// is recorded in the config for Settings to show.
  Future<int> syncIfDue() async {
    final now = clock();
    final last = _lastBackgroundAttempt;
    if (last != null && now.difference(last) < minimumInterval) return 0;
    try {
      if (!await _client.isConfigured()) return 0;
      _lastBackgroundAttempt = now;
      return await syncNow();
    } catch (error) {
      _log.info('Background Lifestats intake sync failed: $error');
      return 0;
    }
  }

  /// Syncs now and returns how many days were sent. Throws
  /// [LifestatsException] when Lifestats is off, unreachable or answers
  /// with an error; the failure is also recorded in the config.
  ///
  /// A call made while a sync is running joins that sync.
  Future<int> syncNow() => _inFlight ??= _sync().whenComplete(() {
    _inFlight = null;
  });

  Future<int> _sync() async {
    final now = clock();
    final config = await _configRepository.getConfig();
    final offset = config.dayStartOffsetTotalMinutes;
    final intakes = (await _intakeRepository.getAllIntakesDBO())
        .map(IntakeEntity.fromIntakeDBO)
        .toList();
    final days = buildDays(
      intakes,
      today: IntakeTimeCalc.logicalDayOf(now, offset),
      offsetTotalMinutes: offset,
    );
    try {
      for (final day in days) {
        await _client.putJson(day.path, day.toJson());
      }
    } on LifestatsException catch (e) {
      if (e.kind != LifestatsErrorKind.notConfigured) {
        await _configRepository.setLifestatsIntakeSyncFailed(
          now,
          e.kind.name,
          e.statusCode,
        );
      }
      rethrow;
    }
    await _configRepository.setLifestatsIntakeSyncSucceeded(now);
    _log.fine('Sent ${days.length} days of caffeine and nicotine');
    return days.length;
  }

  /// The [days] logical days ending with [today], oldest first, each with
  /// the caffeine and nicotine of every intake filed under it.
  static List<LifestatsIntakeDay> buildDays(
    List<IntakeEntity> intakes, {
    required DateTime today,
    required int offsetTotalMinutes,
    int days = windowDays,
  }) {
    final byDay = <DateTime, List<IntakeEntity>>{};
    for (final intake in intakes) {
      final day = IntakeTimeCalc.logicalDayOf(
        intake.dateTime,
        offsetTotalMinutes,
      );
      byDay.putIfAbsent(day, () => []).add(intake);
    }
    List<LifestatsTimedAmount> listOf(
      List<IntakeEntity> dayIntakes,
      double? Function(MealNutrimentsEntity n) per100,
    ) => [
      for (final intake in dayIntakes)
        ?timedAmount(intake, per100(intake.meal.nutriments)),
    ];

    final result = <LifestatsIntakeDay>[];
    for (var back = days - 1; back >= 0; back--) {
      final day = DateTime(today.year, today.month, today.day - back);
      final dayIntakes = byDay[day] ?? const <IntakeEntity>[];
      result.add(
        LifestatsIntakeDay(
          day,
          listOf(dayIntakes, (n) => n.caffeine100),
          listOf(dayIntakes, (n) => n.nicotine100),
        ),
      );
    }
    return result;
  }

  /// [intake]'s amount of a nutrient carried at [per100] mg per 100 g/ml,
  /// rounded to whole mg, or null when the food has none or it rounds to 0
  /// (Lifestats accepts 1 mg and up).
  static LifestatsTimedAmount? timedAmount(
    IntakeEntity intake,
    double? per100,
  ) {
    if (per100 == null) return null;
    final mg = intake.amount * per100 / 100;
    if (!mg.isFinite) return null;
    final rounded = mg.round();
    if (rounded < 1) return null;
    return LifestatsTimedAmount(wallClockTime(intake.dateTime), rounded);
  }

  /// `HH:MM:SS` local time of [dateTime], or null for a day label.
  static String? wallClockTime(DateTime dateTime) {
    if (!IntakeTimeCalc.hasClockTime(dateTime)) return null;
    final local = dateTime.toLocal();
    return '${_twoDigits(local.hour)}:${_twoDigits(local.minute)}:'
        '${_twoDigits(local.second)}';
  }
}

// By hand rather than through DateFormat, whose digits follow the locale.
String _twoDigits(int value) => value.toString().padLeft(2, '0');
