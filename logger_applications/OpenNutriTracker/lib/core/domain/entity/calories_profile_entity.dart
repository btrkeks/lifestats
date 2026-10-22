import 'package:flutter/material.dart';
import 'package:opennutritracker/core/data/dbo/calories_profile_dbo.dart';
import 'package:opennutritracker/generated/l10n.dart';

/// Optional calorie-reference profile for non-binary users. Decouples self
/// identification (`UserGenderEntity`) from the formula reference used in
/// TDEE/BMR/PAL calculations.
enum CaloriesProfileEntity {
  averaged,
  estrogenTypical,
  testosteroneTypical;

  factory CaloriesProfileEntity.fromDBO(CaloriesProfileDBO dbo) {
    switch (dbo) {
      case CaloriesProfileDBO.averaged:
        return CaloriesProfileEntity.averaged;
      case CaloriesProfileDBO.estrogenTypical:
        return CaloriesProfileEntity.estrogenTypical;
      case CaloriesProfileDBO.testosteroneTypical:
        return CaloriesProfileEntity.testosteroneTypical;
    }
  }

  String getName(BuildContext context) {
    switch (this) {
      case CaloriesProfileEntity.averaged:
        return S.of(context).caloriesProfileAveragedLabel;
      case CaloriesProfileEntity.estrogenTypical:
        return S.of(context).caloriesProfileEstrogenTypicalLabel;
      case CaloriesProfileEntity.testosteroneTypical:
        return S.of(context).caloriesProfileTestosteroneTypicalLabel;
    }
  }
}
