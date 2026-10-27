import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:hive_ce/hive.dart';
import 'package:path_provider_platform_interface/path_provider_platform_interface.dart';
import 'package:opennutritracker/core/data/data_source/intake_data_source.dart';
import 'package:opennutritracker/core/data/data_source/tracked_day_data_source.dart';
import 'package:opennutritracker/core/data/repository/intake_repository.dart';
import 'package:opennutritracker/core/data/repository/tracked_day_repository.dart';
import 'package:opennutritracker/core/domain/entity/user_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_type_entity.dart';
import 'package:opennutritracker/core/domain/usecase/add_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/add_tracked_day_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/delete_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_kcal_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_macro_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/log_intake_usecase.dart';
import 'package:opennutritracker/core/utils/hive_db_provider.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_nutriments_entity.dart';

import '../helpers/fake_hive_db_provider.dart';

class _PathProvider extends PathProviderPlatform {
  final String path;
  _PathProvider(this.path);
  @override
  Future<String> getApplicationDocumentsPath() async => path;
}

class _KcalGoal extends Fake implements GetKcalGoalUsecase {
  @override
  Future<double> getKcalGoal({
    UserEntity? userEntity,
    double? totalKcalActivitiesParam,
    double? kcalUserAdjustment,
  }) async => 2000;
}

class _MacroGoal extends Fake implements GetMacroGoalUsecase {
  @override
  Future<double> getCarbsGoal(double totalCalorieGoal) async => 250;
  @override
  Future<double> getFatsGoal(double totalCalorieGoal) async => 70;
  @override
  Future<double> getProteinsGoal(double totalCalorieGoal) async => 100;
}

LogIntakeUsecase _writer(HiveDBProvider db) {
  final intakes = IntakeRepository(IntakeDataSource(db));
  return LogIntakeUsecase(
    AddIntakeUsecase(intakes),
    GetIntakeUsecase(intakes),
    DeleteIntakeUsecase(intakes),
    AddTrackedDayUsecase(TrackedDayRepository(TrackedDayDataSource(db))),
    _KcalGoal(),
    _MacroGoal(),
    db,
  );
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late Directory dir;
  late HiveDBProvider realDb;
  late LogIntakeUsecase writer;
  late PathProviderPlatform original;
  final day = DateTime(2026, 10, 27, 12);
  final meal = MealEntity(
    code: 'quark',
    name: 'Quark',
    url: null,
    mealQuantity: '100',
    mealUnit: 'g',
    servingQuantity: 250,
    servingUnit: 'g',
    servingSize: null,
    nutriments: const MealNutrimentsEntity(
      energyKcal100: 67,
      proteins100: 12,
      carbohydrates100: 4,
      fat100: 0.2,
      sugars100: null,
      saturatedFat100: null,
      fiber100: null,
    ),
    source: MealSourceEntity.custom,
  );
  LogIntakeRequest request(
    String? id, {
    double amount = 250,
    String? profile,
  }) => LogIntakeRequest(
    intakeId: id,
    profileId: profile,
    meal: meal,
    type: IntakeTypeEntity.lunch,
    day: day,
    baseAmount: amount,
    unit: 'g',
  );

  setUp(() async {
    dir = await Directory.systemTemp.createTemp('ont-log-');
    original = PathProviderPlatform.instance;
    PathProviderPlatform.instance = _PathProvider(dir.path);
    realDb = HiveDBProvider();
    await realDb.initHiveDB(Uint8List.fromList(List.filled(32, 7)));
    await realDb.openProfileBoxes('a', '');
    writer = _writer(
      FakeHiveDBProvider(
        intakeBox: realDb.intakeBox,
        trackedDayBox: realDb.trackedDayBox,
      ),
    );
  });

  tearDown(() async {
    await Hive.close();
    realDb.dispose();
    PathProviderPlatform.instance = original;
    await dir.delete(recursive: true);
  });

  test('log persists a receipt and all day totals', () async {
    final receipt = await writer.log(request(null));
    expect(receipt.created, isTrue);
    expect(receipt.day, day);
    expect(realDb.intakeBox.values.single.id, receipt.intakeId);
    final totals = realDb.trackedDayBox.values.single;
    expect(totals.caloriesTracked, 167.5);
    expect(totals.carbsTracked, 10);
    expect(totals.fatTracked, 0.5);
    expect(totals.proteinTracked, 30);
    expect(totals.calorieGoal, 2000);
  });

  test('20 concurrent logs preserve every calorie and macro', () async {
    await Future.wait(List.generate(20, (i) => writer.log(request('id-$i'))));
    expect(realDb.intakeBox.length, 20);
    final totals = realDb.trackedDayBox.values.single;
    expect(totals.caloriesTracked, 3350);
    expect(totals.carbsTracked, 200);
    expect(totals.fatTracked, 10);
    expect(totals.proteinTracked, 600);
  });

  test(
    'concurrent replay writes nothing and returns the original intake',
    () async {
      final receipts = await Future.wait([
        writer.log(request('same')),
        writer.log(request('same', amount: 100)),
      ]);
      expect(receipts.map((r) => r.created), [true, false]);
      expect(receipts.last.intake.amount, 250);
      expect(realDb.intakeBox.length, 1);
      expect(realDb.trackedDayBox.values.single.caloriesTracked, 167.5);
    },
  );

  test('revert twice subtracts only its own totals once', () async {
    await writer.log(request('keep', amount: 100));
    await writer.log(request('undo'));
    expect(await writer.revert('undo'), isTrue);
    expect(await writer.revert('undo'), isFalse);
    expect(realDb.intakeBox.values.single.id, 'keep');
    final totals = realDb.trackedDayBox.values.single;
    expect(totals.caloriesTracked, 67);
    expect(totals.carbsTracked, 4);
    expect(totals.fatTracked, closeTo(0.2, 1e-10));
    expect(totals.proteinTracked, 12);
  });

  test('in-app Add retains support for 50 servings of 250 g', () async {
    final receipt = await writer.log(request('large-serving', amount: 12500));
    expect(receipt.intake.amount, 12500);
    expect(realDb.intakeBox.values.single.amount, 12500);
    expect(realDb.trackedDayBox.values.single.caloriesTracked, 8375);
  });

  test(
    'log started during switch waits and writes only the new profile',
    () async {
      final writer = _writer(realDb);
      var switched = false;
      realDb.addListener(() => switched = true);
      final switching = realDb.switchProfile('b', 'b');
      expect(switched, isFalse);
      final logging = writer.log(request('after-switch')).then((receipt) {
        expect(switched, isTrue);
        return receipt;
      });
      await switching;
      final receipt = await logging;
      expect(receipt.profileId, 'b');
      expect(realDb.intakeBox.values.single.id, 'after-switch');
      await realDb.switchProfile('a', '');
      expect(realDb.intakeBox.isEmpty, isTrue);
      expect(realDb.trackedDayBox.isEmpty, isTrue);
    },
  );

  test('profile-bound log and revert reject after a queued switch', () async {
    final writer = _writer(realDb);
    final switching = realDb.switchProfile('b', 'b');
    final logging = writer.log(request('stale', profile: 'a'));
    await expectLater(logging, throwsA(isA<StaleIntakeProfile>()));
    await switching;
    await expectLater(
      writer.revert('stale', profileId: 'a'),
      throwsA(isA<StaleIntakeProfile>()),
    );
    expect(realDb.intakeBox.isEmpty, isTrue);
    expect(realDb.trackedDayBox.isEmpty, isTrue);
  });
}
