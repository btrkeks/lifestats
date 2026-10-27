import 'package:flutter/widgets.dart';

import 'package:opennutritracker/core/data/repository/intake_repository.dart';
import 'package:opennutritracker/core/domain/entity/config_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/usecase/get_config_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_kcal_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_macro_goal_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/get_user_usecase.dart';
import 'package:opennutritracker/core/domain/usecase/log_intake_usecase.dart';
import 'package:opennutritracker/core/l10n/app_locales.dart';
import 'package:opennutritracker/core/utils/app_locale_service.dart';
import 'package:opennutritracker/core/utils/app_locale_sync.dart';
import 'package:opennutritracker/core/utils/calc/day_boundary_calc.dart';
import 'package:opennutritracker/core/utils/hive_db_provider.dart';
import 'package:opennutritracker/core/utils/meal_type_suggester.dart';
import 'package:opennutritracker/features/quick_log/domain/food_key.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_log_amount.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_log_command.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_log_snapshot.dart';
import 'package:opennutritracker/features/quick_log/domain/suggestion_ranking.dart';
import 'package:opennutritracker/features/quick_log/picker/picker_request.dart';
import 'package:opennutritracker/generated/l10n.dart';

/// The strings the widget is drawn in: the language the app itself would
/// show, resolved the way `main()` resolves it (the OS per-app language
/// first, then the saved choice, then the device).
Future<S> widgetStringsFor(ConfigEntity config) async {
  final locales = appLocales(S.supportedLocales);
  final code =
      supportedLanguageCode(
        await AppLocaleService.getApplicationLocale(),
        locales,
      ) ??
      config.selectedLocale;
  return lookupS(
    basicLocaleListResolution([
      code != null
          ? Locale(code)
          : WidgetsBinding.instance.platformDispatcher.locale,
    ], locales),
  );
}

/// Runs widget commands against storage and assembles the widget snapshot.
class QuickLogService {
  final HiveDBProvider _db;
  final IntakeRepository _intakes;
  final LogIntakeUsecase _logIntake;
  final GetConfigUsecase _config;
  final GetUserUsecase _user;
  final GetKcalGoalUsecase _kcalGoal;
  final GetMacroGoalUsecase _macroGoal;
  final Future<S> Function(ConfigEntity config) _strings;
  final DateTime Function() _clock;

  QuickLogService(
    this._db,
    this._intakes,
    this._logIntake,
    this._config,
    this._user,
    this._kcalGoal,
    this._macroGoal, {
    Future<S> Function(ConfigEntity config) strings = widgetStringsFor,
    DateTime Function() clock = DateTime.now,
  }) : _strings = strings,
       _clock = clock;

  /// Expected refusals come back as [QuickLogRejected]; storage failures
  /// throw.
  Future<QuickLogResult> run(QuickLogCommand command) async {
    switch (command) {
      case RefreshCommand():
        return const QuickLogOk();
      case UndoCommand(:final opId, :final profileId):
        try {
          await _logIntake.revert(opId, profileId: profileId);
        } on StaleIntakeProfile {
          return const QuickLogRejected(RejectReason.staleProfile);
        }
        return const QuickLogOk();
      case LogCommand():
        return _log(command);
    }
  }

  Future<QuickLogResult> _log(LogCommand command) async {
    if (!isValidQuickLogAmount(command.baseAmount)) {
      return const QuickLogRejected(RejectReason.invalid);
    }
    final prepared = await _db.intakeWriteLock.synchronized(() async {
      if (command.profileId != _db.activeProfileId) {
        return (request: null, text: null, reason: RejectReason.staleProfile);
      }
      if (!await _user.hasUserData()) {
        return (request: null, text: null, reason: RejectReason.notSetUp);
      }
      final latest = _latestOf(command.foodKey, await _allIntakes());
      if (latest == null) {
        return (request: null, text: null, reason: RejectReason.unknownFood);
      }
      final s = await _strings(await _config.getConfig());
      final protein = latest.meal.nutriments.proteinsPerUnit;
      return (
        request: LogIntakeRequest(
          intakeId: command.opId,
          profileId: command.profileId,
          meal: latest.meal,
          type: MealTypeSuggester.suggestFromTime(command.tappedAt),
          day: command.tappedAt,
          loggedAt: command.tappedAt,
          baseAmount: command.baseAmount,
          unit: command.unit,
        ),
        text: protein == null
            ? s.quickLogAddedNeutralLabel
            : addedTextFor(command.baseAmount * protein, s),
        reason: null,
      );
    });
    if (prepared.reason != null) return QuickLogRejected(prepared.reason!);
    try {
      await _logIntake.log(prepared.request!);
    } on StaleIntakeProfile {
      return const QuickLogRejected(RejectReason.staleProfile);
    }
    return QuickLogOk(addedText: prepared.text);
  }

  /// The same inputs, in the same order, as Home's `LoadItemsEvent`.
  Future<QuickLogSnapshot> snapshot() =>
      _db.intakeWriteLock.synchronized(_snapshot);

  Future<QuickLogSnapshot> _snapshot() async {
    final config = await _config.getConfig();
    final s = await _strings(config);
    if (!await _user.hasUserData()) return QuickLogNeedsSetup(s.appTitle);
    final now = _clock();
    final offset = config.dayStartOffsetTotalMinutes;
    final today = DayBoundaryCalc.logicalDayOfMinutes(now, offset);
    final intakes = await _allIntakes();
    final todays = intakes.where(
      (intake) => DayBoundaryCalc.isMomentInLogicalDayMinutes(
        today,
        intake.dateTime,
        offset,
      ),
    );
    final user = await _user.getUserData();
    final kcalGoal = await _kcalGoal.getKcalGoal(userEntity: user);
    final proteinGoal = await _macroGoal.getProteinsGoal(kcalGoal);
    return buildQuickLogSnapshot(
      QuickLogInputs(
        profileId: _db.activeProfileId,
        now: now,
        today: today,
        dayStartOffsetMinutes: offset,
        nextDayStart: DateTime(
          today.year,
          today.month,
          today.day + 1,
          0,
          offset,
        ),
        kcalGoal: kcalGoal,
        kcalEaten: todays.fold(0.0, (sum, intake) => sum + intake.totalKcal),
        proteinGoal: proteinGoal,
        proteinEaten: todays.fold(
          0.0,
          (sum, intake) => sum + intake.totalProteinsGram,
        ),
        usesKilojoules: config.usesKilojoules,
        usesImperialFoodUnits: config.usesImperialFoodUnits,
        suggestions: rankSuggestions(
          intakes: intakes,
          today: today,
          dayStartOffsetMinutes: offset,
        ),
        appearance: PickerAppearance(
          localeTag: s.localeName.replaceAll('_', '-'),
          themeMode: config.appTheme.name,
          useMaterialYou: config.useMaterialYou,
          accentColor: config.accentColor,
        ),
      ),
      s,
    );
  }

  Future<List<IntakeEntity>> _allIntakes() async =>
      (await _intakes.getAllIntakesDBO())
          .map(IntakeEntity.fromIntakeDBO)
          .toList();

  static IntakeEntity? _latestOf(FoodKey key, List<IntakeEntity> intakes) {
    IntakeEntity? latest;
    for (final intake in intakes) {
      if (FoodKey.of(intake.meal) != key) continue;
      if (latest == null || intake.dateTime.isAfter(latest.dateTime)) {
        latest = intake;
      }
    }
    return latest;
  }
}
