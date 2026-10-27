import 'dart:developer';
import 'dart:isolate';

import 'package:flutter/services.dart';
import 'package:logging/logging.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/usecase/add_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/add_tracked_day_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_intake_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_kcal_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_macro_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_tracked_day_usecase.dart';
import 'package:opennutritracker/core/utils/id_generator.dart';
import 'package:opennutritracker/core/utils/locator.dart';

/// Debug builds only: the Dart end of `EngineProbeReceiver`. Logs one intake
/// through the process engine with no activity attached, repeating the most
/// recent intake through the same steps as `MealDetailBloc.addIntake`, and
/// answers with what it wrote; or looks an intake up by id.
///
/// Remove with the receiver once the widget's own actions exercise this path
/// (phase 2, U8).
class EngineProbe {
  static const _channel = MethodChannel('com.opennutritracker/debug_probe');
  static final _log = Logger('EngineProbe');

  static void install() {
    _channel.setMethodCallHandler((call) async {
      final arguments = call.arguments as Map?;
      switch (call.method) {
        case 'logIntake':
          final delayMs = arguments?['delayMs'] as int? ?? 0;
          return _logIntake(Duration(milliseconds: delayMs));
        case 'findIntake':
          return _findIntake(arguments?['id'] as String);
        default:
          throw MissingPluginException();
      }
    });
  }

  static Future<Map<String, Object?>> _logIntake(Duration delay) async {
    final isolate = Service.getIsolateId(Isolate.current);
    final recent = await locator<GetIntakeUsecase>().getRecentIntake();
    if (recent.isEmpty) {
      throw PlatformException(code: 'no_recent_intake');
    }
    if (delay > Duration.zero) {
      _log.info('holding the write for $delay (isolate=$isolate)');
      await Future<void>.delayed(delay);
    }
    final template = recent.first;
    final day = DateTime.now();
    final intake = IntakeEntity(
      id: IdGenerator.getUniqueID(),
      unit: template.unit,
      amount: template.amount,
      type: template.type,
      meal: template.meal,
      dateTime: day,
    );
    await locator<AddIntakeUsecase>().addIntake(intake);
    await _updateTrackedDay(intake, day);
    final tracked = await locator<GetTrackedDayUsecase>().getTrackedDay(day);
    final reply = {
      'intakeId': intake.id,
      'meal': intake.meal.name,
      'kcal': intake.totalKcal,
      'dayKcal': tracked?.caloriesTracked,
      'isolate': isolate,
    };
    _log.info('committed $reply');
    return reply;
  }

  static Future<Map<String, Object?>?> _findIntake(String id) async {
    final intake = await locator<GetIntakeUsecase>().getIntakeById(id);
    if (intake == null) return null;
    final tracked = await locator<GetTrackedDayUsecase>().getTrackedDay(
      intake.dateTime,
    );
    return {
      'intakeId': intake.id,
      'meal': intake.meal.name,
      'type': intake.type.name,
      'dateTime': intake.dateTime.toIso8601String(),
      'kcal': intake.totalKcal,
      'dayKcal': tracked?.caloriesTracked,
    };
  }

  static Future<void> _updateTrackedDay(
    IntakeEntity intake,
    DateTime day,
  ) async {
    final trackedDays = locator<AddTrackedDayUsecase>();
    if (!await trackedDays.hasTrackedDay(day)) {
      final kcalGoal = await locator<GetKcalGoalUsecase>().getKcalGoal();
      final macroGoals = locator<GetMacroGoalUsecase>();
      await trackedDays.addNewTrackedDay(
        day,
        kcalGoal,
        await macroGoals.getCarbsGoal(kcalGoal),
        await macroGoals.getFatsGoal(kcalGoal),
        await macroGoals.getProteinsGoal(kcalGoal),
      );
    }
    await trackedDays.addDayCaloriesTracked(day, intake.totalKcal);
    await trackedDays.addDayMacrosTracked(
      day,
      carbsTracked: intake.totalCarbsGram,
      fatTracked: intake.totalFatsGram,
      proteinTracked: intake.totalProteinsGram,
    );
  }
}
