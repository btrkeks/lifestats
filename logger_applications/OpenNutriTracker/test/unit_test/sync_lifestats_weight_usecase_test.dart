import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:opennutritracker/core/data/data_source/lifestats/lifestats_client.dart';
import 'package:opennutritracker/core/data/repository/config_repository.dart';
import 'package:opennutritracker/core/data/repository/user_repository.dart';
import 'package:opennutritracker/core/data/repository/weight_log_repository.dart';
import 'package:opennutritracker/core/domain/entity/app_theme_entity.dart';
import 'package:opennutritracker/core/domain/entity/config_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_gender_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_pal_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_weight_goal_entity.dart';
import 'package:opennutritracker/core/domain/entity/weight_log_entity.dart';
import 'package:opennutritracker/core/domain/usecase/add_weight_log_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/sync_lifestats_weight_usecase.dart';
import 'package:opennutritracker/core/utils/extensions.dart';

class _FakeConfigRepository extends Fake implements ConfigRepository {
  String? baseUrl = 'http://10.0.2.2:3000';
  DateTime? succeededAt;
  (DateTime, String, int?)? failure;

  @override
  Future<ConfigEntity> getConfig() async => ConfigEntity(
    false,
    false,
    false,
    AppThemeEntity.system,
    lifestatsBaseUrl: baseUrl,
  );

  @override
  Future<void> setLifestatsSyncSucceeded(DateTime at) async {
    succeededAt = at;
    failure = null;
  }

  @override
  Future<void> setLifestatsSyncFailed(
    DateTime at,
    String error,
    int? statusCode,
  ) async {
    failure = (at, error, statusCode);
  }
}

/// Keyed by day like the Hive box, so a second write for a date replaces
/// the first.
class _FakeWeightLogRepository extends Fake implements WeightLogRepository {
  final entries = <String, WeightLogEntity>{};
  int writes = 0;

  @override
  Future<void> addEntry(WeightLogEntity entry) async {
    writes++;
    entries[entry.date.toParsedDay()] = entry;
  }

  @override
  Future<WeightLogEntity?> getEntry(DateTime date) async =>
      entries[date.toParsedDay()];
}

class _FakeUserRepository extends Fake implements UserRepository {
  UserEntity user = UserEntity(
    birthday: DateTime(1995, 4, 1),
    heightCM: 183,
    weightKG: 85,
    gender: UserGenderEntity.male,
    goal: UserWeightGoalEntity.maintainWeight,
    pal: UserPALEntity.active,
  );

  @override
  Future<UserEntity> getUserData() async => user;

  @override
  Future<void> updateUserData(UserEntity userEntity) async => user = userEntity;
}

void main() {
  // Lifestats answers in grams for calendar days. AddWeightLogUsecase checks
  // "today" against the real clock, so the fixture is anchored on it.
  final now = DateTime.now();
  final today = DateTime(now.year, now.month, now.day);
  DateTime daysAgo(int n) => DateTime(today.year, today.month, today.day - n);
  String iso(DateTime d) => d.toParsedDay();

  late _FakeConfigRepository config;
  late _FakeWeightLogRepository weightLog;
  late _FakeUserRepository users;
  late List<Uri> requests;
  late Map<DateTime, int?> lifestatsDays;
  late http.Response Function()? failWith;

  SyncLifestatsWeightUsecase build() {
    final client = LifestatsClient(
      MockClient((request) async {
        requests.add(request.url);
        if (failWith != null) return failWith!();
        return http.Response(
          jsonEncode({
            'days': [
              for (final entry in lifestatsDays.entries)
                {'date': iso(entry.key), 'value': entry.value},
            ],
          }),
          200,
        );
      }),
      () async => config.baseUrl,
    );
    return SyncLifestatsWeightUsecase(
      client,
      config,
      weightLog,
      AddWeightLogUsecase(weightLog, users),
    );
  }

  setUp(() {
    config = _FakeConfigRepository();
    weightLog = _FakeWeightLogRepository();
    users = _FakeUserRepository();
    requests = [];
    lifestatsDays = {};
    failWith = null;
    SyncLifestatsWeightUsecase.clock = () => now;
    addTearDown(() => SyncLifestatsWeightUsecase.clock = DateTime.now);
  });

  test('asks for statistic 136 over the last 30 days', () async {
    await build().syncNow();

    expect(requests.single.path, '/statistics/136/achievements');
    expect(requests.single.queryParameters, {
      'start': iso(daysAgo(30)),
      'end': iso(today),
    });
  });

  test('writes one entry per day with a reading, in kg', () async {
    lifestatsDays = {daysAgo(2): 82350, daysAgo(1): null, today: 82100};

    final written = await build().syncNow();

    expect(written, 2);
    expect(
      weightLog.entries.keys,
      unorderedEquals([iso(daysAgo(2)), iso(today)]),
    );
    expect(weightLog.entries[iso(daysAgo(2))]!.weightKg, 82.35);
    expect(weightLog.entries[iso(today)]!.weightKg, 82.1);
    expect(config.succeededAt, now);
  });

  test("today's reading also updates the profile weight", () async {
    lifestatsDays = {today: 80400};

    await build().syncNow();

    expect(users.user.weightKG, 80.4);
  });

  test(
    'overwrites a manual entry on the same day and keeps its note',
    () async {
      weightLog.entries[iso(daysAgo(3))] = WeightLogEntity(
        date: daysAgo(3),
        weightKg: 84.0,
        note: 'after dinner',
      );
      lifestatsDays = {daysAgo(3): 82900};

      await build().syncNow();

      final entry = weightLog.entries[iso(daysAgo(3))]!;
      expect(entry.weightKg, 82.9);
      expect(entry.note, 'after dinner');
    },
  );

  test('keeps manual entries on days without a reading', () async {
    final manual = WeightLogEntity(date: daysAgo(5), weightKg: 83.0);
    weightLog.entries[iso(daysAgo(5))] = manual;
    lifestatsDays = {daysAgo(4): 82800, daysAgo(5): null};

    await build().syncNow();

    expect(weightLog.entries[iso(daysAgo(5))], manual);
  });

  test('a repeated sync writes nothing', () async {
    lifestatsDays = {daysAgo(1): 82350, today: 82100};
    final sut = build();

    expect(await sut.syncNow(), 2);
    expect(await sut.syncNow(), 0);
    expect(weightLog.writes, 2);
  });

  test('ignores values outside what a scale can report', () async {
    lifestatsDays = {daysAgo(1): 0, today: 600000};

    expect(await build().syncNow(), 0);
    expect(weightLog.entries, isEmpty);
  });

  test('records the failure and rethrows on an HTTP error', () async {
    failWith = () => http.Response('{"error":"boom"}', 500);

    await expectLater(build().syncNow(), throwsA(isA<LifestatsException>()));
    expect(config.failure, (now, 'http', 500));
    expect(config.succeededAt, isNull);
  });

  group('syncIfDue', () {
    test('does nothing while no address is set', () async {
      config.baseUrl = null;

      expect(await build().syncIfDue(), 0);
      expect(requests, isEmpty);
      expect(config.failure, isNull);
    });

    test('swallows failures but still records them', () async {
      failWith = () => http.Response('', 503);

      expect(await build().syncIfDue(), 0);
      expect(config.failure?.$2, 'http');
    });

    test('runs at most once per interval', () async {
      final sut = build();
      await sut.syncIfDue();
      SyncLifestatsWeightUsecase.clock = () =>
          now.add(const Duration(minutes: 1));
      await sut.syncIfDue();
      expect(requests, hasLength(1));

      SyncLifestatsWeightUsecase.clock = () =>
          now.add(SyncLifestatsWeightUsecase.minimumInterval);
      await sut.syncIfDue();
      expect(requests, hasLength(2));
    });
  });
}
