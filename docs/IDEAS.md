


- Allow creating monthly goals and draw plots for them!!

- Should the mean score use percentages or scores?
  - Using percentages would allow to get a green boundary by doing a ton a a single day
  - Should probably be a setting as this seems to be entirely depend on personal preference

- Notifaction when HeartbeatLogger starts and ends?
- HeartbeatLogger stopwatch in Taskbar?

- Ask Claude about software architecture and software design
- Proper tests in `db_events.rs`
- application shouldn't panic on db read errors

## Core
- Fix Logging after 00:00 for same day, e.g. went to sleep at 00:30
    - Use DateTimes in TimeFrameList
- Make the domain model "oblivious to how logging data is fetched or stored"

## Heartbeat
- Check if end time is logged correctly even if the heartbeat has expired a long time ago (i.e. max should be last_beat + timeout)

## Stopwatch
- Create stopwatches server-side
- Allow listening to running stopwatches (Implement into task bar)
- Allow to automatically stop stopwatches, if e.g. internet usage is detected

## Fitbit
- Make fitbit module into a proper crate, i.e.
    - store key safely
    - automatically get a key

## Database
- Use Diesel custom types for TimeFrameList
- Cache data read from the db

## Statistics
- Add a "TimeStarted" statistic for Stopwatch / MIDI Logger (Heartbeat Logger)

## Plots
- Fix title not centered
- Implement heatmaps
  - Mainly for Habits, but can also be used for IntegerAmountStatistics
- use stat id for coloring in timeline? Or allow manual specification
- Fix data is read multiple times from the database when drawing plots
- Add 折れ線グラフ
	- Smoothen the graph with a smoothing kernel / filter
		- pub struct SmoothingKernel { x_dist: u64 } impl SmoothingKernel { pub fn apply(y_values: &[u64]) }


## Web interface
- Rewrite using Next.js frontend

- When changing the date, the goal doesn't change accordingly
- Add a "Reflect" page
  - Show what was good and what went bad that day

## CLI logging
- Give a warning if logging for a time in the future

Long term:
- Weekly plan (different goal for every week day)

- monthly / yearly stats
- Monthly / yearly goals

Ideas:
- Add a statistic that logs the time the first data was logged for another stat
	- e.g. "Time of first internet usage today"
- write a phone application for logging:
  - phone unlocks (time and total number)
  - app usage
- Stopwatch: Play a sound when a certain time has been reached (e.g. 1h)
- Calendar application where you can take TimeSpans statistics and drop them in the calendar to create goals

- Allow attaching tags to specific days for the statistics
  - E.g.:
    - "Ate right before sleep"/"Drank alcohol before sleep" for amount of sleep statistic
  
- Suggest turning on the programming logger if RustRover is focused for example
  - "I see you are doing X. Do you want to activate the stopwatch logger for X?"

- Allow importing csv files

- Run server on raspberry pi