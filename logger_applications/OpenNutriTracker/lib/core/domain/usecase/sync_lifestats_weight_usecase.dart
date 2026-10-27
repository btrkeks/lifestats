import 'package:logging/logging.dart';
import 'package:opennutritracker/core/data/data_source/lifestats/lifestats_client.dart';
import 'package:opennutritracker/core/data/repository/config_repository.dart';
import 'package:opennutritracker/core/data/repository/weight_log_repository.dart';
import 'package:opennutritracker/core/domain/entity/weight_log_entity.dart';
import 'package:opennutritracker/core/domain/usecase/add_weight_log_usecase.dart';

/// Copies scale readings from Lifestats into the weight log.
///
/// A Beurer scale syncs to the Lifestats statistic [weightStatisticId],
/// stored in grams, one level per day. Each day with a reading in the last
/// [lookbackDays] becomes that day's weight-log entry in kg, overwriting
/// whatever the log held for the date. Days without a reading are left
/// alone, so manual entries there survive. Days whose entry already holds
/// the reading are skipped, which makes a repeat sync write nothing.
class SyncLifestatsWeightUsecase {
  /// `[Beurer] Weight (kg)`, stored in grams.
  static const weightStatisticId = 136;
  static const lookbackDays = 30;

  /// The shortest gap between two background syncs. Resumes come in bursts,
  /// and an unreachable server costs a full timeout each time.
  static const minimumInterval = Duration(minutes: 5);

  /// The clock "today" and the sync timestamps are read from. Replaced in
  /// tests.
  static DateTime Function() clock = DateTime.now;

  final _log = Logger('SyncLifestatsWeightUsecase');
  final LifestatsClient _client;
  final ConfigRepository _configRepository;
  final WeightLogRepository _weightLogRepository;
  final AddWeightLogUsecase _addWeightLogUsecase;

  Future<int>? _inFlight;
  DateTime? _lastBackgroundAttempt;

  SyncLifestatsWeightUsecase(
    this._client,
    this._configRepository,
    this._weightLogRepository,
    this._addWeightLogUsecase,
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
      _log.info('Background Lifestats weight sync failed: $error');
      return 0;
    }
  }

  /// Syncs now and returns how many weight-log entries were written. Throws
  /// [LifestatsException] when Lifestats is off, unreachable or answers
  /// with an error; the failure is also recorded in the config.
  ///
  /// A call made while a sync is running joins that sync.
  Future<int> syncNow() => _inFlight ??= _sync().whenComplete(() {
    _inFlight = null;
  });

  Future<int> _sync() async {
    final now = clock();
    final today = DateTime(now.year, now.month, now.day);
    final start = DateTime(today.year, today.month, today.day - lookbackDays);
    final Map<DateTime, num> readings;
    try {
      readings = await _client.getDailyValues(weightStatisticId, start, today);
    } on LifestatsException catch (e) {
      if (e.kind != LifestatsErrorKind.notConfigured) {
        await _configRepository.setLifestatsSyncFailed(
          now,
          e.kind.name,
          e.statusCode,
        );
      }
      rethrow;
    }

    var written = 0;
    final days = readings.keys.toList()..sort();
    for (final day in days) {
      final grams = readings[day]!;
      // The Lifestats route accepts 1..=500000 g; anything else isn't a
      // reading.
      if (grams <= 0 || grams > 500000) continue;
      final weightKg = grams / 1000;
      final existing = await _weightLogRepository.getEntry(day);
      if (existing != null && existing.weightKg == weightKg) continue;
      // Through the use case, so a reading for today also updates the
      // profile weight the way a manual entry does.
      await _addWeightLogUsecase.addEntry(
        WeightLogEntity(date: day, weightKg: weightKg, note: existing?.note),
      );
      written++;
    }
    await _configRepository.setLifestatsSyncSucceeded(now);
    _log.fine('Synced $written weight entries from Lifestats');
    return written;
  }
}
