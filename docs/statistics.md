# Statistic Types

This document lists all statistic types available in `core/domain/model/src/statistic/`.

## 1. Habit (habit.rs)
**Type:** `HabitStatistic`
**Logs:** Boolean values (true/false)
**Purpose:** Track whether a habit was completed on a given day. Examples include daily routines, activities to do or avoid.
**Goal:** Target is a boolean - either the habit should be done (true) or avoided (false).

## 2. Integer Amount (integer_amount.rs)
**Type:** `IntegerAmountStatistic`
**Logs:** Integer values (i64)
**Purpose:** Track countable quantities or amounts. Examples include number of pages read, glasses of water consumed, or any discrete countable metric.
**Goal:** Target is an integer amount with an `is_good` flag indicating whether higher values are better (true) or worse (false).

## 3. Point In Time (point_in_time.rs)
**Type:** `PointInTimeStatistic`
**Logs:** Time of day (MyTime)
**Purpose:** Track when something was completed during the day. Examples include wake-up time, bedtime, or task completion time.
**Goal:** Target time to finish by. Earlier completion yields higher scores, with score decreasing linearly after the target time (1 hour late = 0%).

## 4. Time Amount (time_amount.rs)
**Type:** `TimeAmountStatistic`
**Logs:** Duration/time span (MyDuration)
**Purpose:** Track the amount of time spent on activities. Examples include study time, exercise duration, internet usage, or phone screen time.
**Goal:** Target duration with an `is_good` flag. For good activities (e.g., exercise), higher durations are better. For bad activities (e.g., phone usage), lower durations are better (0% at 2x target).

## 5. Time Spans (time_spans.rs)
**Type:** `TimeSpansStatistic`
**Logs:** Time frames (TimeFrame) stored as lists (TimeFrameList)
**Purpose:** Track specific time periods during the day when an activity occurred. Multiple non-overlapping time spans can be logged per day. Each span has a start and end time.
**Goal:** Target time frame list with an efficiency percentage (0-100). The goal calculates based on total duration across all spans compared to the target duration adjusted by efficiency.
