import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:opennutritracker/core/data/data_source/lifestats/lifestats_client.dart';
import 'package:opennutritracker/core/data/dbo/intake_dbo.dart';
import 'package:opennutritracker/core/data/repository/config_repository.dart';
import 'package:opennutritracker/core/data/repository/intake_repository.dart';
import 'package:opennutritracker/core/domain/entity/app_theme_entity.dart';
import 'package:opennutritracker/core/domain/entity/config_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_type_entity.dart';
import 'package:opennutritracker/core/domain/usecase/sync_lifestats_intake_usecase.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_nutriments_entity.dart';

class _FakeConfigRepository extends Fake implements ConfigRepository {
  String? baseUrl = 'http://10.0.2.2:3000';
  int dayStartOffsetHours = 0;
  DateTime? succeededAt;
  (DateTime, String, int?)? failure;

  @override
  Future<ConfigEntity> getConfig() async => ConfigEntity(
    false,
    false,
    false,
    AppThemeEntity.system,
    lifestatsBaseUrl: baseUrl,
    dayStartOffsetHours: dayStartOffsetHours,
  );

  @override
  Future<void> setLifestatsIntakeSyncSucceeded(DateTime at) async {
    succeededAt = at;
    failure = null;
  }

  @override
  Future<void> setLifestatsIntakeSyncFailed(
    DateTime at,
    String error,
    int? statusCode,
  ) async {
    failure = (at, error, statusCode);
  }
}

class _FakeIntakeRepository extends Fake implements IntakeRepository {
  final intakes = <IntakeEntity>[];

  @override
  Future<List<IntakeDBO>> getAllIntakesDBO() async =>
      intakes.map(IntakeDBO.fromIntakeEntity).toList();
}

IntakeEntity _intake(
  String id,
  DateTime dateTime,
  double amount, {
  double? caffeine,
  double? nicotine,
}) => IntakeEntity(
  id: id,
  unit: 'g',
  amount: amount,
  type: IntakeTypeEntity.snack,
  dateTime: dateTime,
  meal: MealEntity(
    code: id,
    name: id,
    url: null,
    mealQuantity: '100',
    mealUnit: 'g',
    servingQuantity: null,
    servingUnit: 'g',
    servingSize: null,
    nutriments: MealNutrimentsEntity(
      energyKcal100: 2,
      carbohydrates100: 0,
      fat100: 0,
      proteins100: 0,
      sugars100: null,
      saturatedFat100: null,
      fiber100: null,
      caffeine100: caffeine,
      nicotine100: nicotine,
    ),
    source: MealSourceEntity.custom,
  ),
);

void main() {
  final today = DateTime(2026, 10, 27);

  group('buildDays', () {
    List<LifestatsIntakeDay> build(
      List<IntakeEntity> intakes, {
      int offset = 0,
    }) => SyncLifestatsIntakeUsecase.buildDays(
      intakes,
      today: today,
      offsetTotalMinutes: offset,
    );

    test('covers the last 14 logical days, oldest first, today last', () {
      final days = build(const []);
      expect(days, hasLength(14));
      expect(days.first.path, '/integrations/opennutritracker/days/2026-10-14');
      expect(days.last.path, '/integrations/opennutritracker/days/2026-10-27');
    });

    test('a day without intake is two empty lists, a confirmed zero', () {
      expect(build(const []).first.toJson(), {
        'caffeine': <Object>[],
        'nicotine': <Object>[],
      });
    });

    test('amount × per-100 value ÷ 100, rounded to whole mg', () {
      final days = build([
        // 200 ml coffee at 40 mg / 100 ml.
        _intake('coffee', DateTime(2026, 10, 27, 8, 12, 30), 200, caffeine: 40),
        // A 1.5 g gum at 133.3 mg / 100 g: 1.9995 mg.
        _intake('gum', DateTime(2026, 10, 27, 14, 3), 1.5, nicotine: 133.3),
        // Exactly 60.5 mg rounds half up.
        _intake('tea', DateTime(2026, 10, 27, 16, 0), 100, caffeine: 60.5),
      ]);
      expect(
        jsonEncode(days.last.toJson()),
        jsonEncode({
          'caffeine': [
            {'time': '08:12:30', 'amount_mg': 80},
            {'time': '16:00:00', 'amount_mg': 61},
          ],
          'nicotine': [
            {'time': '14:03:00', 'amount_mg': 2},
          ],
        }),
      );
    });

    test('foods without the nutrient, or under half a mg, send nothing', () {
      final days = build([
        _intake('bread', DateTime(2026, 10, 26, 7), 80),
        _intake('chocolate', DateTime(2026, 10, 26, 15), 1, caffeine: 40),
      ]);
      expect(days[12].day, DateTime(2026, 10, 26));
      expect(days[12].caffeine, isEmpty);
      expect(days[12].nicotine, isEmpty);
    });

    test('an intake that only knows its day gets time null', () {
      final days = build([
        _intake('legacy', DateTime.utc(2026, 10, 20), 200, caffeine: 40),
      ]);
      expect(days[6].day, DateTime(2026, 10, 20));
      expect(days[6].caffeine, [const LifestatsTimedAmount(null, 80)]);
    });

    test('files by logical day and sends the real clock time', () {
      // 02:30 on the 27th, with the day starting at 04:00, is the 26th.
      final days = build([
        _intake('espresso', DateTime(2026, 10, 27, 2, 30), 30, caffeine: 210),
      ], offset: 4 * 60);
      expect(days[12].caffeine, [const LifestatsTimedAmount('02:30:00', 63)]);
      expect(days[13].caffeine, isEmpty);
    });

    test('intakes outside the window are left out', () {
      final days = build([
        _intake('old', DateTime(2026, 10, 13, 9), 200, caffeine: 40),
        _intake('future', DateTime(2026, 10, 28, 9), 200, caffeine: 40),
      ]);
      expect(days.expand((d) => d.caffeine), isEmpty);
    });
  });

  group('sync', () {
    late _FakeConfigRepository config;
    late _FakeIntakeRepository intakes;
    late List<http.Request> requests;
    late http.Response Function(http.Request request) respond;

    SyncLifestatsIntakeUsecase build() => SyncLifestatsIntakeUsecase(
      LifestatsClient(
        MockClient((request) async {
          requests.add(request);
          return respond(request);
        }),
        () async => config.baseUrl,
      ),
      config,
      intakes,
    );

    setUp(() {
      config = _FakeConfigRepository();
      intakes = _FakeIntakeRepository();
      requests = [];
      respond = (_) => http.Response('', 204);
      SyncLifestatsIntakeUsecase.clock = () => DateTime(2026, 10, 27, 21, 5);
      addTearDown(() => SyncLifestatsIntakeUsecase.clock = DateTime.now);
    });

    test('PUTs each day of the window in order and records success', () async {
      intakes.intakes.add(
        _intake('coffee', DateTime(2026, 10, 27, 8, 12), 200, caffeine: 40),
      );

      final sent = await build().syncNow();

      expect(sent, 14);
      expect(requests.map((r) => r.method).toSet(), {'PUT'});
      expect(
        requests.first.url.toString(),
        'http://10.0.2.2:3000/integrations/opennutritracker/days/2026-10-14',
      );
      expect(requests.last.url.path, endsWith('/2026-10-27'));
      expect(
        requests.last.headers['Content-Type'],
        startsWith('application/json'),
      );
      expect(jsonDecode(requests.last.body), {
        'caffeine': [
          {'time': '08:12:00', 'amount_mg': 80},
        ],
        'nicotine': <Object>[],
      });
      expect(config.succeededAt, DateTime(2026, 10, 27, 21, 5));
    });

    test('stops at the first failure and records it', () async {
      respond = (_) => http.Response(
        '{"error":{"code":"not_found","message":"no route"}}',
        404,
      );

      await expectLater(
        build().syncNow(),
        throwsA(
          isA<LifestatsException>().having((e) => e.statusCode, 'status', 404),
        ),
      );
      expect(requests, hasLength(1));
      expect(config.failure, (DateTime(2026, 10, 27, 21, 5), 'http', 404));
      expect(config.succeededAt, isNull);
    });

    test(
      'the background sync is quiet, off without an address, and throttled',
      () async {
        config.baseUrl = null;
        final sut = build();
        expect(await sut.syncIfDue(), 0);
        expect(requests, isEmpty);

        config.baseUrl = 'http://10.0.2.2:3000';
        respond = (_) => http.Response('', 500);
        expect(await sut.syncIfDue(), 0);
        expect(config.failure?.$3, 500);

        respond = (_) => http.Response('', 204);
        requests.clear();
        expect(await sut.syncIfDue(), 0);
        expect(requests, isEmpty);
      },
    );
  });
}
