import 'package:opennutritracker/core/data/repository/config_repository.dart';
import 'package:opennutritracker/core/data/repository/user_repository.dart';
import 'package:opennutritracker/core/data/repository/weight_log_repository.dart';
import 'package:opennutritracker/core/domain/entity/config_entity.dart';
import 'package:opennutritracker/core/utils/calc/day_boundary_calc.dart';
import 'package:opennutritracker/core/utils/calc/macro_calc.dart';
import 'package:opennutritracker/core/utils/calc/protein_goal_calc.dart';

/// Daily macro goals in grams.
///
/// Protein is a factor (g per kg, a user setting) times the 7-day average
/// body weight from the weight log, or the profile weight when the log has
/// no recent reading. Carbs and fat split the calories protein leaves over,
/// in the ratio of their stored percentages.
class GetMacroGoalUsecase {
  final ConfigRepository _configRepository;
  final UserRepository _userRepository;
  final WeightLogRepository _weightLogRepository;

  GetMacroGoalUsecase(
    this._configRepository,
    this._userRepository,
    this._weightLogRepository,
  );

  Future<double> getCarbsGoal(double totalCalorieGoal) async {
    final config = await _configRepository.getConfig();
    return MacroCalc.getTotalCarbsGoal(
      totalCalorieGoal,
      await _proteinGoal(config),
      userCarbsGoal: config.userCarbGoalPct,
      userFatsGoal: config.userFatGoalPct,
    );
  }

  Future<double> getFatsGoal(double totalCalorieGoal) async {
    final config = await _configRepository.getConfig();
    return MacroCalc.getTotalFatsGoal(
      totalCalorieGoal,
      await _proteinGoal(config),
      userCarbsGoal: config.userCarbGoalPct,
      userFatsGoal: config.userFatGoalPct,
    );
  }

  /// [totalCalorieGoal] is unused: protein follows body weight, not energy.
  /// The parameter stays so every macro getter is called the same way.
  Future<double> getProteinsGoal(double totalCalorieGoal) async =>
      _proteinGoal(await _configRepository.getConfig());

  /// The body weight today's protein goal is scaled from.
  Future<ProteinReferenceWeight> getProteinReferenceWeight() async =>
      _referenceWeight(await _configRepository.getConfig());

  Future<double> _proteinGoal(ConfigEntity config) async =>
      ProteinGoalCalc.goalGrams(
        (await _referenceWeight(config)).weightKg,
        gramsPerKg: config.userProteinGramsPerKg,
      );

  Future<ProteinReferenceWeight> _referenceWeight(ConfigEntity config) async {
    final user = await _userRepository.getUserData();
    return ProteinGoalCalc.referenceWeight(
      await _weightLogRepository.getAllEntries(),
      today: DayBoundaryCalc.currentLogicalDayLabel(
        config.dayStartOffsetHours,
        config.dayStartOffsetMinutes,
      ),
      profileWeightKg: user.weightKG,
    );
  }
}
