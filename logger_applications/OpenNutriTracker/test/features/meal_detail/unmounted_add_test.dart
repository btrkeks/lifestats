import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:get_it/get_it.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_type_entity.dart';
import 'package:opennutritracker/core/domain/usecase/get_intake_usecase.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_nutriments_entity.dart';
import 'package:opennutritracker/features/diary/presentation/bloc/calendar_day_bloc.dart';
import 'package:opennutritracker/features/diary/presentation/bloc/diary_bloc.dart';
import 'package:opennutritracker/features/home/presentation/bloc/home_bloc.dart';
import 'package:opennutritracker/features/meal_detail/presentation/bloc/meal_detail_bloc.dart';
import 'package:opennutritracker/features/meal_detail/presentation/widgets/meal_detail_bottom_sheet.dart';
import 'package:opennutritracker/generated/l10n.dart';

class _Detail extends Fake implements MealDetailBloc {
  final committed = Completer<void>();
  bool started = false;
  @override
  MealDetailState get state =>
      const MealDetailInitial(totalQuantityConverted: '100', selectedUnit: 'g');
  @override
  Future<void> addIntake(
    BuildContext context,
    String unit,
    String amountText,
    IntakeTypeEntity type,
    MealEntity meal,
    DateTime day,
  ) async {
    started = true;
    await committed.future;
  }
}

class _Intakes extends Fake implements GetIntakeUsecase {
  @override
  Future<List<IntakeEntity>> getBreakfastIntakeByDay(
    DateTime day, {
    int dayStartOffsetHours = 0,
    int dayStartOffsetMinutes = 0,
  }) async => [];
}

class _Home extends Fake implements HomeBloc {
  final events = <HomeEvent>[];
  @override
  void add(HomeEvent event) => events.add(event);
}

class _Diary extends Fake implements DiaryBloc {
  final events = <DiaryEvent>[];
  @override
  void add(DiaryEvent event) => events.add(event);
}

class _Calendar extends Fake implements CalendarDayBloc {
  final events = <CalendarDayEvent>[];
  @override
  void add(CalendarDayEvent event) => events.add(event);
}

void main() {
  testWidgets(
    'a committed Add refreshes all pages even after its sheet unmounts',
    (tester) async {
      final detail = _Detail();
      final home = _Home();
      final diary = _Diary();
      final calendar = _Calendar();
      final locator = GetIt.instance;
      locator.registerSingleton<GetIntakeUsecase>(_Intakes());
      locator.registerSingleton<HomeBloc>(home);
      locator.registerSingleton<DiaryBloc>(diary);
      locator.registerSingleton<CalendarDayBloc>(calendar);
      addTearDown(locator.reset);
      final quantity = TextEditingController(text: '100');
      addTearDown(quantity.dispose);
      final meal = MealEntity(
        code: 'test',
        name: 'Quark',
        url: null,
        mealQuantity: '100',
        mealUnit: 'g',
        servingQuantity: null,
        servingUnit: null,
        servingSize: null,
        source: MealSourceEntity.custom,
        nutriments: const MealNutrimentsEntity(
          energyKcal100: 67,
          proteins100: 12,
          carbohydrates100: 4,
          fat100: 0.2,
          sugars100: null,
          saturatedFat100: null,
          fiber100: null,
        ),
      );
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [S.delegate],
          supportedLocales: S.supportedLocales,
          home: Scaffold(
            body: MealDetailBottomSheet(
              product: meal,
              day: DateTime(2026, 10, 27),
              intakeTypeEntity: IntakeTypeEntity.breakfast,
              quantityTextController: quantity,
              mealDetailBloc: detail,
              selectedUnit: 'g',
              onQuantityOrUnitChanged: (_, _) {},
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Add'));
      await tester.pump();
      expect(detail.started, isTrue);
      await tester.pumpWidget(const SizedBox());
      detail.committed.complete();
      await tester.pump();
      expect(home.events, [isA<LoadItemsEvent>()]);
      expect(diary.events, [isA<LoadDiaryYearEvent>()]);
      expect(calendar.events, [isA<RefreshCalendarDayEvent>()]);
      expect(tester.takeException(), isNull);
    },
  );
}
