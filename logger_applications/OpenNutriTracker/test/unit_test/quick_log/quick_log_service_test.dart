import 'dart:convert';
import 'dart:ui' show Locale;
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hive_ce/hive.dart';
import 'package:opennutritracker/core/data/data_source/intake_data_source.dart';
import 'package:opennutritracker/core/data/data_source/tracked_day_data_source.dart';
import 'package:opennutritracker/core/data/dbo/intake_dbo.dart';
import 'package:opennutritracker/core/data/dbo/tracked_day_dbo.dart';
import 'package:opennutritracker/core/data/repository/intake_repository.dart';
import 'package:opennutritracker/core/data/repository/tracked_day_repository.dart';
import 'package:opennutritracker/core/domain/entity/app_theme_entity.dart';
import 'package:opennutritracker/core/domain/entity/config_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_type_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_entity.dart';
import 'package:opennutritracker/core/domain/usecase/add_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/add_tracked_day_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/delete_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_config_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_kcal_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_macro_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_user_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/log_intake_usecase.dart';
import 'package:opennutritracker/core/utils/hive_db_provider.dart';
import 'package:opennutritracker/features/quick_log/domain/food_key.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_log_command.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_log_snapshot.dart';
import 'package:opennutritracker/features/quick_log/quick_log_bridge.dart';
import 'package:opennutritracker/features/quick_log/quick_log_service.dart';
import 'package:opennutritracker/generated/l10n.dart';
import 'package:path_provider_platform_interface/path_provider_platform_interface.dart';

import 'quick_log_test_data.dart';

class _PathProvider extends PathProviderPlatform {
  final String path;
  _PathProvider(this.path);
  @override
  Future<String> getApplicationDocumentsPath() async => path;
}

class _Config extends Fake implements GetConfigUsecase {
  @override
  Future<ConfigEntity> getConfig() async =>
      const ConfigEntity(true, true, false, AppThemeEntity.system);
}

class _User extends Fake implements GetUserUsecase {
  bool initialized = true;
  @override
  Future<bool> hasUserData() async => initialized;
  @override
  Future<UserEntity> getUserData() async => _UserEntity();
}

class _UserEntity extends Fake implements UserEntity {}

class _KcalGoal extends Fake implements GetKcalGoalUsecase {
  @override
  Future<double> getKcalGoal({
    UserEntity? userEntity,
    double? totalKcalActivitiesParam,
    double? kcalUserAdjustment,
  }) async => 2400;
}

class _MacroGoal extends Fake implements GetMacroGoalUsecase {
  @override
  Future<double> getCarbsGoal(double totalCalorieGoal) async => 250;
  @override
  Future<double> getFatsGoal(double totalCalorieGoal) async => 70;
  @override
  Future<double> getProteinsGoal(double totalCalorieGoal) async => 150;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late Directory dir;
  late PathProviderPlatform original;
  late HiveDBProvider db;
  late _User user;
  late QuickLogService service;
  final now = DateTime(2026, 10, 27, 20);
  final yesterday = DateTime(2026, 10, 26, 12);
  final quark = food(code: '4005500000000', name: 'Speisequark');
  final quarkKey = FoodKey.of(quark)!;

  Future<void> seed(IntakeEntity intake) =>
      db.intakeBox.add(IntakeDBO.fromIntakeEntity(intake));

  LogCommand log(
    String opId, {
    FoodKey? key,
    String profile = 'a',
    double amount = 250,
    DateTime? at,
  }) => LogCommand(
    opId: opId,
    foodKey: key ?? quarkKey,
    profileId: profile,
    baseAmount: amount,
    unit: 'g',
    tappedAt: at ?? now,
  );

  setUp(() async {
    dir = await Directory.systemTemp.createTemp('ont-quick-log-');
    original = PathProviderPlatform.instance;
    PathProviderPlatform.instance = _PathProvider(dir.path);
    db = HiveDBProvider();
    await db.initHiveDB(Uint8List.fromList(List.filled(32, 7)));
    await db.openProfileBoxes('a', '');
    user = _User();
    final intakes = IntakeRepository(IntakeDataSource(db));
    service = QuickLogService(
      db,
      intakes,
      LogIntakeUsecase(
        AddIntakeUsecase(intakes),
        GetIntakeUsecase(intakes),
        DeleteIntakeUsecase(intakes),
        AddTrackedDayUsecase(TrackedDayRepository(TrackedDayDataSource(db))),
        _KcalGoal(),
        _MacroGoal(),
        db,
      ),
      _Config(),
      user,
      _KcalGoal(),
      _MacroGoal(),
      strings: (_) async => lookupS(const Locale('en')),
      clock: () => now,
    );
    await seed(intake('old', yesterday, meal: quark, amount: 200));
  });

  tearDown(() async {
    await Hive.close();
    db.dispose();
    PathProviderPlatform.instance = original;
    await dir.delete(recursive: true);
  });

  test(
    'log writes the latest meal under the tap time and replays as a no-op',
    () async {
      final breakfast = DateTime(2026, 10, 27, 8, 5);
      final result = await service.run(log('op-1', at: breakfast));
      expect(result.toWire(), {'status': 'ok', 'addedText': 'Added · +30 g'});
      expect(
        (await service.run(log('op-1', at: breakfast))).toWire()['status'],
        'ok',
      );
      final written = db.intakeBox.values.singleWhere((i) => i.id == 'op-1');
      expect(db.intakeBox.length, 2);
      expect(written.amount, 250);
      expect(written.dateTime, breakfast);
      expect(
        IntakeEntity.fromIntakeDBO(written).type,
        IntakeTypeEntity.breakfast,
      );
      expect(db.trackedDayBox.values.single.proteinTracked, 30);
    },
  );

  test(
    'undo removes the intake and its totals; a second undo is a no-op',
    () async {
      await service.run(log('op-1'));
      for (var i = 0; i < 2; i++) {
        final result = await service.run(
          const UndoCommand(opId: 'op-1', profileId: 'a'),
        );
        expect(result.toWire()['status'], 'ok');
      }
      expect(db.intakeBox.values.map((i) => i.id), ['old']);
      expect(db.trackedDayBox.values.single.proteinTracked, 0);
      expect(db.trackedDayBox.values.single.caloriesTracked, 0);
    },
  );

  test('refusals write nothing', () async {
    Future<Object?> reason(QuickLogCommand command) async =>
        (await service.run(command)).toWire()['reason'];

    expect(await reason(log('stale', profile: 'b')), 'staleProfile');
    expect(
      await reason(log('unknown', key: FoodKey.of(food(code: 'other'))!)),
      'unknownFood',
    );
    expect(await reason(log('zero', amount: 0)), 'invalid');
    expect(
      await reason(const UndoCommand(opId: 'old', profileId: 'b')),
      'staleProfile',
    );
    user.initialized = false;
    expect(await reason(log('setup')), 'notSetUp');
    expect(db.intakeBox.values.map((i) => i.id), ['old']);
  });

  test('snapshot shows today\'s totals and the ranked foods', () async {
    await service.run(log('op-1'));
    final snapshot = await service.snapshot() as QuickLogReady;
    expect(snapshot.profileId, 'a');
    expect(snapshot.logicalDay, '2026-10-27');
    expect(snapshot.status.proteinText, '30/150 g');
    expect(snapshot.rows.single.title, 'Speisequark');
    expect(snapshot.rows.single.pillText, '+ 30 g');

    user.initialized = false;
    expect(await service.snapshot(), isA<QuickLogNeedsSetup>());
  });

  test('wire commands parse, and malformed ones are null', () {
    final parsed = QuickLogCommand.fromWire({
      'op': 'log',
      'opId': 'x',
      'foodKey': quarkKey.value,
      'profileId': 'a',
      'baseAmount': 250,
      'unit': 'g',
      'tappedAtMs': now.millisecondsSinceEpoch,
    });
    expect(parsed, isA<LogCommand>());
    expect((parsed as LogCommand).tappedAt, now);
    expect(QuickLogCommand.fromWire({'op': 'refresh'}), isA<RefreshCommand>());
    for (final bad in <Object?>[
      null,
      {'op': 'nope'},
      {'op': 'undo', 'opId': 'x'},
      {
        'op': 'log',
        'opId': 'x',
        'foodKey': 'bogus',
        'profileId': 'a',
        'baseAmount': 1,
        'unit': 'g',
        'tappedAtMs': 0,
      },
      {
        'op': 'log',
        'opId': 'x',
        'foodKey': quarkKey.value,
        'profileId': 'a',
        'baseAmount': double.nan,
        'unit': 'g',
        'tappedAtMs': 0,
      },
    ]) {
      expect(QuickLogCommand.fromWire(bad), isNull, reason: '$bad');
    }
  });

  group('bridge', () {
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    late List<String> events;

    setUp(() {
      events = [];
      messenger.setMockMethodCallHandler(QuickLogBridge.channel, (call) async {
        if (call.method == 'publish') {
          final json = jsonDecode(call.arguments as String) as Map;
          events.add('publish ${(json['status'] as Map)['proteinText']}');
        }
        return null;
      });
    });

    tearDown(
      () => messenger.setMockMethodCallHandler(QuickLogBridge.channel, null),
    );

    Future<Map<Object?, Object?>> run(Map<String, Object?> command) async {
      const codec = StandardMethodCodec();
      final reply = await messenger.handlePlatformMessage(
        QuickLogBridge.channel.name,
        codec.encodeMethodCall(MethodCall('run', command)),
        null,
      );
      final result = codec.decodeEnvelope(reply!) as Map<Object?, Object?>;
      events.add('reply ${result['status']}');
      return result;
    }

    test('publishes the new snapshot before it replies', () async {
      final bridge = QuickLogBridge(service, db)..install();
      addTearDown(bridge.dispose);
      await run({
        'op': 'log',
        'opId': 'op-1',
        'foodKey': quarkKey.value,
        'profileId': 'a',
        'baseAmount': 250.0,
        'unit': 'g',
        'tappedAtMs': now.millisecondsSinceEpoch,
      });
      expect(events, ['publish 0/150 g', 'publish 30/150 g', 'reply ok']);
      expect((await run({'op': 'bogus'}))['reason'], 'invalid');
    });
  });

  group('publisher', () {
    test(
      'publishes once per burst of writes, and keeps watching after a profile switch',
      () async {
        var published = 0;
        final publisher = QuickLogPublisher(
          db,
          () async => published++,
          debounce: const Duration(milliseconds: 20),
        )..start();
        addTearDown(publisher.stop);

        await seed(intake('w1', now, meal: quark));
        await seed(intake('w2', now, meal: quark));
        await Future<void>.delayed(const Duration(milliseconds: 80));
        expect(published, 1);

        await db.switchProfile('b', 'b');
        await Future<void>.delayed(const Duration(milliseconds: 80));
        expect(published, 2);

        await seed(intake('w3', now, meal: quark));
        await Future<void>.delayed(const Duration(milliseconds: 80));
        expect(published, 3);

        await db.trackedDayBox.add(
          TrackedDayDBO(day: now, calorieGoal: 2400, caloriesTracked: 168),
        );
        await Future<void>.delayed(const Duration(milliseconds: 80));
        expect(published, 4);
      },
    );
  });
}
