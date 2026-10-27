import 'package:flutter_test/flutter_test.dart';
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
import 'package:opennutritracker/core/domain/usecase/get_macro_goal_usecase.dart';
import 'package:opennutritracker/core/utils/calc/day_boundary_calc.dart';

class _FakeConfigRepository extends Fake implements ConfigRepository {
  final ConfigEntity config;

  _FakeConfigRepository(this.config);

  @override
  Future<ConfigEntity> getConfig() async => config;
}

class _FakeUserRepository extends Fake implements UserRepository {
  final UserEntity user;

  _FakeUserRepository(this.user);

  @override
  Future<UserEntity> getUserData() async => user;
}

class _FakeWeightLogRepository extends Fake implements WeightLogRepository {
  final List<WeightLogEntity> entries;

  _FakeWeightLogRepository(this.entries);

  @override
  Future<List<WeightLogEntity>> getAllEntries() async => entries;
}

void main() {
  final now = DateTime(2026, 10, 27, 12);
  final user = UserEntity(
    birthday: DateTime(1995, 4, 1),
    heightCM: 183,
    weightKG: 85.0,
    gender: UserGenderEntity.male,
    goal: UserWeightGoalEntity.maintainWeight,
    pal: UserPALEntity.active,
  );

  GetMacroGoalUsecase usecase({
    ConfigEntity config = const ConfigEntity(
      false,
      false,
      false,
      AppThemeEntity.system,
    ),
    List<WeightLogEntity> weighIns = const [],
  }) => GetMacroGoalUsecase(
    _FakeConfigRepository(config),
    _FakeUserRepository(user),
    _FakeWeightLogRepository(weighIns),
  );

  setUp(() {
    DayBoundaryCalc.clock = () => now;
    addTearDown(() => DayBoundaryCalc.clock = DateTime.now);
  });

  test('protein uses the profile weight when nothing was weighed', () async {
    // 1.8 × 85 = 153 → 155 g.
    expect(await usecase().getProteinsGoal(2800), 155);
  });

  test('protein follows the 7-day average of the weight log', () async {
    final goal = await usecase(
      weighIns: [
        WeightLogEntity(date: DateTime(2026, 10, 27), weightKg: 80.2),
        WeightLogEntity(date: DateTime(2026, 10, 24), weightKg: 80.6),
      ],
    ).getProteinsGoal(2800);

    // 1.8 × 80.4 = 144.72 → 145 g.
    expect(goal, 145);
  });

  test('protein does not depend on the calorie goal', () async {
    final sut = usecase();

    expect(await sut.getProteinsGoal(1800), await sut.getProteinsGoal(3200));
  });

  test('protein uses the stored g/kg factor', () async {
    final goal = await usecase(
      config: const ConfigEntity(
        false,
        false,
        false,
        AppThemeEntity.system,
        userProteinGramsPerKg: 2.2,
      ),
    ).getProteinsGoal(2800);

    // 2.2 × 85 = 187 → 185 g.
    expect(goal, 185);
  });

  test(
    'carbs and fat split what protein leaves, in the stored ratio',
    () async {
      final sut = usecase(
        config: const ConfigEntity(
          false,
          false,
          false,
          AppThemeEntity.system,
          userCarbGoalPct: 0.5,
          userFatGoalPct: 0.3,
        ),
      );

      // 2800 − 155 × 4 = 2180 kcal; carbs 50:30 → 62.5 %.
      expect(await sut.getCarbsGoal(2800), closeTo(2180 * 0.625 / 4, 1e-9));
      expect(await sut.getFatsGoal(2800), closeTo(2180 * 0.375 / 9, 1e-9));
    },
  );
}
