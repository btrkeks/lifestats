# LifeStats: Personal Observability Platform

## What It Is

LifeStats is a comprehensive personal tracking system that captures habits, time allocation, wellness metrics, and performance data. It's designed for quantified-self enthusiasts who want data-driven insights into their daily routines and progress toward goals.

## What It Tracks

### Core Metric Types

1. **Habits** - Boolean daily tracking (done/not done)
   - Examples: Exercised today, meditated, avoided junk food, took vitamins

2. **Counts** - Discrete quantities
   - Examples: Pages read, flashcards studied, glasses of water, push-ups completed

3. **Durations** - Total time spent on activities
   - Examples: Study time, exercise duration, phone screen time, reading time

4. **Timestamps** - When specific events occurred
   - Examples: Wake-up time, bedtime, task completion time

5. **Time Intervals** - Specific work sessions with start/end times
   - Examples: Deep work sessions, focused study periods, practice intervals
   - Supports multiple sessions per day with automatic aggregation

### Derived Metrics
- Automatic summation across multiple activities
- Time differences between events (e.g., time in bed = bedtime to wake-up)

## How Data Gets Collected

### Automatic Tracking
- **Anki Integration** - Automatically logs flashcard study sessions per deck, including time and interruptions
- **MIDI Logger** - Detects piano/keyboard practice through MIDI input
- **Fitbit Sync** - Pulls health data (steps, sleep, activity) via OAuth integration
- **Browser Extension** - Tracks website usage and time online (Chrome/Edge)

### Manual Tracking
- **Stopwatch Logger** - Interactive terminal stopwatch for focused work sessions
- **CLI Logger** - Quick command-line logging for on-the-spot entries
- **HTTP API** - Programmatic access for custom integrations

## Insights & Outputs

### Goal Management
- Set **daily goals** for specific dates or **default goals** as baseline targets
- Goals adapt to metric type:
  - Target amounts (with "higher/lower is better" orientation)
  - Target times (scored by punctuality)
  - Target durations (with efficiency percentages)
  - Boolean targets (do this / avoid this)

### Visualization & Analytics
- Plot generation for time-series visualization
- Daily summary timelines showing activity patterns
- Category allocation analysis (where time goes)
- Desktop wallpaper updates with current data
- Historical data preservation with backward compatibility

### Smart Features
- **Idle detection** - Automatically pauses tracking during inactivity
- **Session tracking** - Measures disruptions and quality of focus periods
- **Real-time updates** - Visualizations update immediately when data changes
- **Privacy-first** - All data stored locally in SQLite (no cloud dependency)

## Coaching Applications

### For Building Better Habits
- Track consistency of daily routines over time
- Identify patterns in habit adherence
- Set progressive goals for habit formation
- Visualize streaks and setbacks

### For Time Management
- See total time allocation across all activities
- Identify time wasters (excessive internet, phone usage)
- Balance productive vs. unproductive activities
- Track deep work vs. shallow work ratios

### For Skill Development
- Monitor deliberate practice hours (e.g., music practice via MIDI)
- Track study effectiveness (Anki integration)
- Count learning inputs (pages read, courses completed)
- Measure consistency of practice routines

### For Health & Wellness
- Sync wearable data (Fitbit) for sleep and activity
- Track wake/sleep consistency
- Monitor exercise frequency and duration
- Correlate wellness metrics with performance data

### For Productivity Optimization
- Measure focused work sessions vs. fragmented time
- Track interruptions and context switches
- Identify peak performance hours
- Set and monitor realistic daily targets

### For Self-Awareness
- Comprehensive activity timelines show how days actually unfold
- Quantified data reveals gaps between intention and reality
- Historical trends expose patterns not visible day-to-day
- Data-driven reflection enables targeted behavior change

## Key Strengths for Performance Coaching

1. **Multi-modal data collection** - Combines automatic sensing (wearables, software) with intentional tracking (manual logs)
2. **Flexible metric types** - Tracks both yes/no habits and nuanced time/quantity data
3. **Goal-oriented** - Built around setting targets and measuring progress
4. **Comprehensive coverage** - From sleep to screen time to skill practice
5. **Privacy-focused** - Client owns their data, stored locally
6. **Developer-friendly** - Extensible for custom tracking needs

## Current Status

LifeStats is a working system currently in active development (mid-refactor). The core functionality is stable:
- HTTP API is fully operational
- Multiple proven logger applications
- Fitbit integration working
- Database with migration support

The system is currently developer-focused rather than end-user ready, but provides powerful capabilities for technically-inclined users committed to quantified self-improvement.

## Bottom Line

LifeStats enables **comprehensive personal observability** - turning daily activities, habits, and metrics into quantified data that supports goal-setting, pattern recognition, and evidence-based behavior change. For performance coaching, it provides the granular data needed to identify leverage points, track interventions, and demonstrate progress over time.
