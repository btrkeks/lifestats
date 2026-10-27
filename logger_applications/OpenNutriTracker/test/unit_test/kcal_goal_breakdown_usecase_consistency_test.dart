import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/data/repository/config_repository.dart';
import 'package:opennutritracker/core/data/repository/user_activity_repository.dart';
import 'package:opennutritracker/core/data/repository/user_repository.dart';
import 'package:opennutritracker/core/data/repository/weight_log_repository.dart';
import 'package:opennutritracker/core/domain/entity/app_theme_entity.dart';
import 'package:opennutritracker/core/domain/entity/config_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_activity_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_gender_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_pal_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_weight_goal_entity.dart';
import 'package:opennutritracker/core/domain/entity/weight_log_entity.dart';
import 'package:opennutritracker/core/domain/usecase/get_kcal_goal_breakdown_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_kcal_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_macro_goal_usecase.dart';

import '../fixture/physical_activity_entity_fixtures.dart';

/// End-to-end consistency of the transparency screen's data source: feed
/// the breakdown usecase and the production goal/macro usecases the same
/// repositories and require identical outputs. If either side ever changes
/// how it fetches or combines inputs (activity date, config fields, macro
/// percentages), this fails before the screen can show a diverging number.
class _FakeUserRepository extends Fake implements UserRepository {
  final UserEntity user;

  _FakeUserRepository(this.user);

  @override
  Future<UserEntity> getUserData() async => user;
}

class _FakeConfigRepository extends Fake implements ConfigRepository {
  final ConfigEntity config;

  _FakeConfigRepository(this.config);

  @override
  Future<ConfigEntity> getConfig() async => config;
}

class _FakeUserActivityRepository extends Fake
    implements UserActivityRepository {
  final List<UserActivityEntity> activities;

  _FakeUserActivityRepository(this.activities);

  @override
  Future<List<UserActivityEntity>> getAllUserActivityByDate(
    DateTime dateTime, {
    int dayStartOffsetHours = 0,
    int dayStartOffsetMinutes = 0,
  }) async => activities;
}

class _FakeWeightLogRepository extends Fake implements WeightLogRepository {
  final List<WeightLogEntity> entries;

  _FakeWeightLogRepository(this.entries);

  @override
  Future<List<WeightLogEntity>> getAllEntries() async => entries;
}

void main() {
  // Captured once so the birthday can't straddle a midnight boundary
  // between multiple DateTime.now() calls, and yesterday (rather than
  // day - 1) so the test still works on the 1st of the month.
  final now = DateTime.now();
  final yesterday = now.subtract(const Duration(days: 1));
  final user = UserEntity(
    birthday: DateTime(yesterday.year - 42, yesterday.month, yesterday.day),
    heightCM: 172.0,
    weightKG: 74.5,
    gender: UserGenderEntity.female,
    goal: UserWeightGoalEntity.loseWeight,
    pal: UserPALEntity.lowActive,
    weeklyWeightGoalKg: -0.25,
    targetWeightKg: 70.0,
    caloriesTaperEnabled: true,
  );

  const config = ConfigEntity(
    false,
    false,
    false,
    AppThemeEntity.system,
    userKcalAdjustment: 120,
    userCarbGoalPct: 0.5,
    userFatGoalPct: 0.3,
    userProteinGramsPerKg: 2.0,
  );

  final today = DateTime(now.year, now.month, now.day);
  final weighIns = [
    WeightLogEntity(date: today, weightKg: 73.8),
    WeightLogEntity(
      date: DateTime(today.year, today.month, today.day - 2),
      weightKg: 74.4,
    ),
  ];

  final activities = [
    UserActivityEntity(
      'a1',
      45,
      312.5,
      now,
      PhysicalActivityFixtures.moderateBicycling,
    ),
    UserActivityEntity(
      'a2',
      20,
      87.25,
      now,
      PhysicalActivityFixtures.lightDancing,
    ),
  ];

  late GetKcalGoalBreakdownUsecase breakdownUsecase;
  late GetKcalGoalUsecase kcalGoalUsecase;
  late GetMacroGoalUsecase macroGoalUsecase;

  setUp(() {
    final userRepo = _FakeUserRepository(user);
    final configRepo = _FakeConfigRepository(config);
    final activityRepo = _FakeUserActivityRepository(activities);
    macroGoalUsecase = GetMacroGoalUsecase(
      configRepo,
      userRepo,
      _FakeWeightLogRepository(weighIns),
    );
    breakdownUsecase = GetKcalGoalBreakdownUsecase(
      userRepo,
      configRepo,
      activityRepo,
      macroGoalUsecase,
    );
    kcalGoalUsecase = GetKcalGoalUsecase(userRepo, configRepo, activityRepo);
  });

  test(
    'breakdown total equals the production kcal goal from the same data',
    () async {
      final breakdown = await breakdownUsecase.getBreakdown();
      final productionGoal = await kcalGoalUsecase.getKcalGoal();

      expect(breakdown.totalKcalGoal, productionGoal);
    },
  );

  test('breakdown activity kcal is the sum of the logged activities', () async {
    final breakdown = await breakdownUsecase.getBreakdown();

    expect(breakdown.activityKcal, closeTo(312.5 + 87.25, 0.0001));
  });

  test('breakdown reads the manual adjustment from config', () async {
    final breakdown = await breakdownUsecase.getBreakdown();

    expect(breakdown.manualKcalAdjustment, 120);
  });

  test('breakdown macro grams equal the production macro goals', () async {
    final breakdown = await breakdownUsecase.getBreakdown();
    final goal = breakdown.totalKcalGoal;

    expect(breakdown.carbsGoalGrams, await macroGoalUsecase.getCarbsGoal(goal));
    expect(breakdown.fatsGoalGrams, await macroGoalUsecase.getFatsGoal(goal));
    expect(
      breakdown.proteinsGoalGrams,
      await macroGoalUsecase.getProteinsGoal(goal),
    );
    // 2.0 g/kg × mean(73.8, 74.4) = 148.2 → 150 g.
    expect(breakdown.proteinsGoalGrams, 150);
    expect(breakdown.proteinWeightSampleCount, 2);
    expect(breakdown.proteinReferenceWeightKg, closeTo(74.1, 1e-9));
    expect(breakdown.carbsShareOfRemainder, 0.5 / 0.8);
  });

  test('breakdown user snapshot mirrors the stored profile', () async {
    final breakdown = await breakdownUsecase.getBreakdown();

    expect(breakdown.age, user.age);
    expect(breakdown.heightCM, 172.0);
    expect(breakdown.weightKG, 74.5);
    expect(breakdown.gender, UserGenderEntity.female);
    expect(breakdown.palCategory, UserPALEntity.lowActive);
    expect(breakdown.goal, UserWeightGoalEntity.loseWeight);
    expect(breakdown.weeklyWeightGoalKg, -0.25);
    expect(breakdown.targetWeightKg, 70.0);
    expect(breakdown.taperEnabled, isTrue);
  });
}
