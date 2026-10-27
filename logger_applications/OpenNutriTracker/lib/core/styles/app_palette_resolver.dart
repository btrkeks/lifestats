import 'package:flutter/material.dart';
import 'package:opennutritracker/core/styles/app_palette.dart';

/// The light and dark palettes for the user's colour settings.
/// [lightDynamicAccent] and [darkDynamicAccent] are the harmonized
/// wallpaper primaries, null where the platform has none.
///
/// The friendly-flat canvas, surfaces and macro colours are fixed; only the
/// single vivid accent follows the user. Material You (when enabled and the
/// platform has wallpaper colours) or a picked accent drives that one role,
/// otherwise the brand green. Shared by the app and the widget's amount
/// picker, which runs in its own engine and must look like the app.
({AppPalette light, AppPalette dark}) resolveAppPalettes({
  required bool useMaterialYou,
  required int? accentColor,
  Color? lightDynamicAccent,
  Color? darkDynamicAccent,
}) {
  if (useMaterialYou &&
      lightDynamicAccent != null &&
      darkDynamicAccent != null) {
    return (
      light: AppPalette.light.withAccent(lightDynamicAccent),
      dark: AppPalette.dark.withAccent(darkDynamicAccent),
    );
  }
  if (accentColor != null) {
    final seed = Color(accentColor);
    return (
      light: AppPalette.light.withAccent(seed),
      dark: AppPalette.dark.withAccent(seed),
    );
  }
  return (light: AppPalette.light, dark: AppPalette.dark);
}
