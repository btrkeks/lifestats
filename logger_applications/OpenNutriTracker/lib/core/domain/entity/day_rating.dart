import 'package:flutter/material.dart';

/// Represents the rating of a tracked day based on calorie goal adherence.
///
/// [good] - User stayed within acceptable calorie tolerance
/// [poor] - User exceeded the acceptable calorie difference threshold
enum DayRating {
  good,
  poor;

  /// Returns the calendar indicator color for this rating.
  Color getCalendarColor(BuildContext context) {
    return switch (this) {
      DayRating.good => Theme.of(context).colorScheme.primary,
      DayRating.poor => Theme.of(context).colorScheme.error,
    };
  }

  /// Returns the text color for this rating.
  Color getTextColor(BuildContext context) {
    return switch (this) {
      DayRating.good => Theme.of(context).colorScheme.onSecondaryContainer,
      DayRating.poor => Theme.of(context).colorScheme.onErrorContainer,
    };
  }

  /// Returns the background color for text with this rating.
  Color getTextBackgroundColor(BuildContext context) {
    return switch (this) {
      DayRating.good => Theme.of(context).colorScheme.secondaryContainer,
      DayRating.poor => Theme.of(context).colorScheme.errorContainer,
    };
  }
}
