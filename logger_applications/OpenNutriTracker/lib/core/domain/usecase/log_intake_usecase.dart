import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_type_entity.dart';
import 'package:opennutritracker/core/domain/usecase/add_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/add_tracked_day_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/delete_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_kcal_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_macro_goal_usecase.dart';
import 'package:opennutritracker/core/utils/hive_db_provider.dart';
import 'package:opennutritracker/core/utils/id_generator.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';

class LogIntakeRequest {
  final String? intakeId;
  final String? profileId;
  final MealEntity meal;
  final IntakeTypeEntity type;
  final DateTime day;
  final double baseAmount;
  final String unit;

  const LogIntakeRequest({
    this.intakeId,
    this.profileId,
    required this.meal,
    required this.type,
    required this.day,
    required this.baseAmount,
    required this.unit,
  });
}

class IntakeReceipt {
  final String intakeId;
  final String profileId;
  final DateTime day;
  final bool created;
  final IntakeEntity intake;

  const IntakeReceipt({
    required this.intakeId,
    required this.profileId,
    required this.day,
    required this.created,
    required this.intake,
  });
}

class StaleIntakeProfile implements Exception {
  const StaleIntakeProfile();
}

class LogIntakeUsecase {
  final AddIntakeUsecase _addIntake;
  final GetIntakeUsecase _getIntake;
  final DeleteIntakeUsecase _deleteIntake;
  final AddTrackedDayUsecase _trackedDay;
  final GetKcalGoalUsecase _kcalGoal;
  final GetMacroGoalUsecase _macroGoal;
  final HiveDBProvider _db;

  LogIntakeUsecase(
    this._addIntake,
    this._getIntake,
    this._deleteIntake,
    this._trackedDay,
    this._kcalGoal,
    this._macroGoal,
    this._db,
  );

  /// The calendar accounting key intentionally retains the app Add semantics.
  Future<IntakeReceipt> log(LogIntakeRequest request) =>
      _db.intakeWriteLock.synchronized(() async {
        _checkProfile(request.profileId);
        if (!request.baseAmount.isFinite ||
            request.baseAmount <= 0 ||
            request.baseAmount > 10000) {
          throw ArgumentError.value(request.baseAmount, 'baseAmount');
        }
        final existing = request.intakeId == null
            ? null
            : await _getIntake.getIntakeById(request.intakeId!);
        if (existing != null) return _receipt(existing, created: false);
        final intake = IntakeEntity(
          id: request.intakeId ?? IdGenerator.getUniqueID(),
          unit: request.unit,
          amount: request.baseAmount,
          type: request.type,
          meal: request.meal,
          dateTime: request.day,
        );
        await _addIntake.addIntake(intake);
        await _accountAdd(intake);
        return _receipt(intake, created: true);
      });

  Future<bool> revert(String intakeId, {String? profileId}) =>
      _db.intakeWriteLock.synchronized(() async {
        _checkProfile(profileId);
        final intake = await _getIntake.getIntakeById(intakeId);
        if (intake == null) return false;
        await _deleteIntake.deleteIntake(intake);
        await _trackedDay.removeDayCaloriesTracked(
          intake.dateTime,
          intake.totalKcal,
        );
        await _trackedDay.removeDayMacrosTracked(
          intake.dateTime,
          carbsTracked: intake.totalCarbsGram,
          fatTracked: intake.totalFatsGram,
          proteinTracked: intake.totalProteinsGram,
        );
        return true;
      });

  void _checkProfile(String? profileId) {
    if (profileId != null && profileId != _db.activeProfileId) {
      throw const StaleIntakeProfile();
    }
  }

  IntakeReceipt _receipt(IntakeEntity intake, {required bool created}) =>
      IntakeReceipt(
        intakeId: intake.id,
        profileId: _db.activeProfileId,
        day: intake.dateTime,
        created: created,
        intake: intake,
      );

  Future<void> _accountAdd(IntakeEntity intake) async {
    final day = intake.dateTime;
    if (!await _trackedDay.hasTrackedDay(day)) {
      final kcal = await _kcalGoal.getKcalGoal();
      await _trackedDay.addNewTrackedDay(
        day,
        kcal,
        await _macroGoal.getCarbsGoal(kcal),
        await _macroGoal.getFatsGoal(kcal),
        await _macroGoal.getProteinsGoal(kcal),
      );
    }
    await _trackedDay.addDayCaloriesTracked(day, intake.totalKcal);
    await _trackedDay.addDayMacrosTracked(
      day,
      carbsTracked: intake.totalCarbsGram,
      fatTracked: intake.totalFatsGram,
      proteinTracked: intake.totalProteinsGram,
    );
  }
}
