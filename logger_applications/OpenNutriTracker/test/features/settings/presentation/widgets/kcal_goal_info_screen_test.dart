import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/domain/entity/calories_profile_entity.dart';
import 'package:opennutritracker/core/domain/entity/kcal_goal_breakdown_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_gender_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_pal_entity.dart';
import 'package:opennutritracker/core/domain/entity/user_weight_goal_entity.dart';
import 'package:opennutritracker/core/domain/usecase/get_kcal_goal_breakdown_usecase.dart';
import 'package:opennutritracker/core/utils/calc/protein_goal_calc.dart';
import 'package:opennutritracker/core/utils/energy_unit_provider.dart';
import 'package:opennutritracker/features/settings/presentation/widgets/kcal_goal_info_screen.dart';
import 'package:opennutritracker/generated/l10n.dart';
import 'package:provider/provider.dart';
import '../../../../helpers/test_l10n.dart';

/// Renders the transparency screen against breakdowns computed by the real
/// [KcalGoalBreakdownEntity.compute] and asserts the *displayed strings*
/// equal independently hand-computed values. This pins the whole chain —
/// math, formatting, and rounding conventions (truncation, matching the
/// dashboard) — so the screen cannot silently drift from the numbers the
/// rest of the app shows.
class _FakeBreakdownUsecase extends Fake
    implements GetKcalGoalBreakdownUsecase {
  final KcalGoalBreakdownEntity breakdown;

  _FakeBreakdownUsecase(this.breakdown);

  @override
  Future<KcalGoalBreakdownEntity> getBreakdown() async => breakdown;
}

UserEntity _buildUser({
  UserGenderEntity gender = UserGenderEntity.male,
  CaloriesProfileEntity? caloriesProfile,
  UserWeightGoalEntity goal = UserWeightGoalEntity.loseWeight,
  UserPALEntity pal = UserPALEntity.active,
  double? weeklyWeightGoalKg,
  double? targetWeightKg,
  bool taperEnabled = false,
  double weightKG = 80.0,
}) {
  // Yesterday (not `day - 1`) so the date stays valid on the 1st of the
  // month; captured once so components can't straddle a midnight rollover.
  final yesterday = DateTime.now().subtract(const Duration(days: 1));
  return UserEntity(
    birthday: DateTime(yesterday.year - 30, yesterday.month, yesterday.day),
    heightCM: 180.0,
    weightKG: weightKG,
    gender: gender,
    goal: goal,
    pal: pal,
    caloriesProfile: caloriesProfile,
    weeklyWeightGoalKg: weeklyWeightGoalKg,
    targetWeightKg: targetWeightKg,
    caloriesTaperEnabled: taperEnabled,
  );
}

Widget _wrap(KcalGoalBreakdownEntity breakdown, {bool usesKilojoules = false}) {
  return ChangeNotifierProvider(
    create: (_) => EnergyUnitProvider(usesKilojoules: usesKilojoules),
    child: MaterialApp(
      localizationsDelegates: const [
        S.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: S.supportedLocales,
      home: KcalGoalInfoScreen(usecase: _FakeBreakdownUsecase(breakdown)),
    ),
  );
}

Future<void> _pump(
  WidgetTester tester,
  KcalGoalBreakdownEntity breakdown, {
  bool usesKilojoules = false,
}) async {
  // Tall viewport so the lazy ListView inflates every card — the tests
  // assert content across the whole screen, including the last cards.
  tester.view.physicalSize = const Size(1200, 4500);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(_wrap(breakdown, usesKilojoules: usesKilojoules));
  await tester.pumpAndSettle();
}

void main() {
  group('KcalGoalInfoScreen', () {
    // Reference case, hand-computed from IOM 2005 p. 204. PA multiplies the
    // weight *and* height terms — see #987, which is why these numbers moved.
    // TDEE = 864 − 9.72×30 + 1.27×(14.2×80 + 503×1.80)
    //      = 572.4 + 1.27×2041.4 = 3164.98
    // Goal = 3164.98 − 500 + 150 + 320 = 3134.98
    KcalGoalBreakdownEntity maleBreakdown() => KcalGoalBreakdownEntity.compute(
      user: _buildUser(),
      userKcalAdjustment: 150,
      totalKcalActivities: 320,
    );

    testWidgets('shows the hand-computed input values', (tester) async {
      await _pump(tester, maleBreakdown());

      expect(find.text('30 years'), findsOneWidget);
      expect(find.text('180 cm'), findsOneWidget);
      // Twice: the inputs card and, with no weigh-ins, the protein card's
      // body weight.
      expect(find.text('80.0 kg'), findsNWidgets(2));
      expect(find.text('Active'), findsWidgets);
      expect(find.text('Lose Weight'), findsOneWidget);
    });

    testWidgets('shows PAL and PA values from the IOM tables', (tester) async {
      await _pump(tester, maleBreakdown());

      expect(find.text('1.75'), findsOneWidget); // PAL for "active"
      expect(find.text('1.27'), findsOneWidget); // male PA at PAL 1.75
    });

    testWidgets('shows the substituted IOM formula and truncated TDEE', (
      tester,
    ) async {
      await _pump(tester, maleBreakdown());

      expect(
        find.textContaining(
          '864 − 9.72 × 30 + 1.27 × (14.2 × 80.0 + 503 × 1.80)',
        ),
        findsOneWidget,
      );
      // Box result truncates exactly like the summary rows do.
      expect(find.textContaining('= 3164 kcal'), findsOneWidget);
      // TDEE appears twice: TDEE card result row + result card line.
      expect(find.text('3164 kcal'), findsNWidgets(2));
    });

    testWidgets('shows every goal component with an explicit sign', (
      tester,
    ) async {
      await _pump(tester, maleBreakdown());

      // Without a taper the same value fills the base row, the applied
      // row, and the result-card line.
      expect(find.text('−500 kcal'), findsNWidgets(3));
      expect(find.text('+150 kcal'), findsNWidgets(2));
      expect(find.text('+320 kcal'), findsNWidgets(2));
      expect(
        find.text(l10nEn.kcalGoalInfoAdjustmentExplanationFlat),
        findsOneWidget,
      );
    });

    testWidgets('total equals the hand-computed goal, truncated like home', (
      tester,
    ) async {
      await _pump(tester, maleBreakdown());

      // 3134.98 truncates to 3134 — identical to the dashboard's toInt().
      expect(find.text('3134 kcal'), findsOneWidget);
    });

    testWidgets('macro card shows the hand-computed gram targets', (
      tester,
    ) async {
      await _pump(tester, maleBreakdown());

      // Protein: 1.8 g/kg × 80 kg profile weight = 144 → 145 g.
      // That leaves 3134.98 − 580 = 2554.98 kcal, split 60:25.
      // Carbs: × 0.706 ÷ 4 = 450.88 → 450 g (truncated, like the dashboard
      // tiles). Fat: × 0.294 ÷ 9 = 83.50 → 83 g.
      expect(
        find.textContaining('1.8 g/kg × 80.0 kg = 144.0 → 145 g'),
        findsOneWidget,
      );
      expect(
        find.textContaining('(3134 − 145 × 4) × 71 % ÷ 4 = 450 g'),
        findsOneWidget,
      );
      expect(
        find.textContaining('(3134 − 145 × 4) × 29 % ÷ 9 = 83 g'),
        findsOneWidget,
      );
      expect(find.text('145 g'), findsOneWidget);
      expect(find.text('450 g'), findsOneWidget);
      expect(find.text('83 g'), findsOneWidget);
      expect(
        find.text(l10nEn.kcalGoalInfoProteinWeightFromProfile),
        findsOneWidget,
      );
    });

    testWidgets('kJ mode converts summary rows but keeps the formula in kcal', (
      tester,
    ) async {
      await _pump(tester, maleBreakdown(), usesKilojoules: true);

      // 3134.98 kcal × 4.184 = 13116.7 → 13116 kJ (truncated).
      expect(find.text('13116 kJ'), findsOneWidget);
      // 3164.98 kcal × 4.184 = 13242.3 → 13242 kJ.
      expect(find.text('13242 kJ'), findsNWidgets(2));
      // The IOM equation is defined in kcal, so the box must stay kcal.
      expect(find.textContaining('= 3164 kcal'), findsOneWidget);
    });

    testWidgets('taper case shows base and tapered adjustment plus the note', (
      tester,
    ) async {
      // 3 kg above a 75 kg target with the taper on → factor 0.5.
      final breakdown = KcalGoalBreakdownEntity.compute(
        user: _buildUser(
          weightKG: 78.0,
          targetWeightKg: 75.0,
          taperEnabled: true,
        ),
        totalKcalActivities: 0,
      );
      await _pump(tester, breakdown);

      expect(find.text('−500 kcal'), findsOneWidget); // base row only
      expect(find.text('−250 kcal'), findsNWidgets(2)); // applied + result
      expect(find.text(l10nEn.kcalGoalInfoTaperNote), findsOneWidget);
    });

    testWidgets('taper note is absent when no taper is active', (tester) async {
      await _pump(tester, maleBreakdown());

      expect(find.text(l10nEn.kcalGoalInfoTaperNote), findsNothing);
    });

    testWidgets('weekly rate shows its explanation and 1100-based adjustment', (
      tester,
    ) async {
      final breakdown = KcalGoalBreakdownEntity.compute(
        user: _buildUser(weeklyWeightGoalKg: -0.5),
        totalKcalActivities: 0,
      );
      await _pump(tester, breakdown);

      // −0.5 kg/week × 1100 = −550 kcal/day (base, applied, result rows).
      expect(find.text('−550 kcal'), findsNWidgets(3));
      expect(
        find.text(l10nEn.kcalGoalInfoAdjustmentExplanationWeekly),
        findsOneWidget,
      );
      expect(
        find.text(l10nEn.kcalGoalInfoAdjustmentExplanationFlat),
        findsNothing,
      );
    });

    testWidgets('female profile uses the female formula constants only', (
      tester,
    ) async {
      final breakdown = KcalGoalBreakdownEntity.compute(
        user: _buildUser(gender: UserGenderEntity.female),
        totalKcalActivities: 0,
      );
      await _pump(tester, breakdown);

      expect(find.textContaining('387 − 7.31 × 30'), findsOneWidget);
      expect(find.textContaining('864 −'), findsNothing);
      expect(find.text(l10nEn.kcalGoalInfoAveragedNote), findsNothing);
    });

    testWidgets('non-binary averaged profile shows both reference sides', (
      tester,
    ) async {
      final breakdown = KcalGoalBreakdownEntity.compute(
        user: _buildUser(gender: UserGenderEntity.nonBinary),
        totalKcalActivities: 0,
      );
      await _pump(tester, breakdown);

      expect(find.textContaining('864 − 9.72 × 30'), findsOneWidget);
      expect(find.textContaining('387 − 7.31 × 30'), findsOneWidget);
      expect(find.text(l10nEn.kcalGoalInfoAveragedNote), findsOneWidget);
      expect(
        find.textContaining(l10nEn.kcalGoalInfoMaleReferenceLabel),
        findsWidgets,
      );
      expect(
        find.textContaining(l10nEn.kcalGoalInfoFemaleReferenceLabel),
        findsWidgets,
      );
    });

    testWidgets('custom factor, weigh-ins and ratio are rendered', (
      tester,
    ) async {
      final breakdown = KcalGoalBreakdownEntity.compute(
        user: _buildUser(),
        totalKcalActivities: 0,
        userCarbsGoalPct: 0.4,
        userFatsGoalPct: 0.3,
        userProteinGramsPerKg: 2.2,
        proteinReferenceWeight: const ProteinReferenceWeight(
          weightKg: 76.4,
          sampleCount: 3,
        ),
      );
      await _pump(tester, breakdown);

      // 2.2 × 76.4 = 168.08 → 170 g; carbs take 40 / 70 = 57 % of the rest.
      expect(
        find.textContaining('2.2 g/kg × 76.4 kg = 168.1 → 170 g'),
        findsOneWidget,
      );
      expect(find.textContaining('× 57 % ÷ 4'), findsOneWidget);
      expect(find.textContaining('× 43 % ÷ 9'), findsOneWidget);
      expect(
        find.text(l10nEn.kcalGoalInfoProteinWeightFromLog(3)),
        findsOneWidget,
      );
    });
  });
}
