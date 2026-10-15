## WorkoutWorker

When a workout is in progress, there are certain responsibilities that sit outside the app’s normal UI-driven flow. These include things like:

- Persistent notifications
- Timers that must continue while the app is backgrounded
- Integration with platform-specific system UI (e.g. foreground notifications on Android)

These responsibilities are **inherently platform-dependent**, both in _how_ they are executed and _what_ they integrate with.

The purpose of the `WorkoutWorker` is to provide a **platform-specific execution environment** that reacts to a standardised stream of workout-related messages (events and commands) emitted by the main application.

The `WorkoutWorker` is **not the authority** for workout state:

- The source of truth lives in the main app (Redux / JS)
- If the app process crashes or is killed, the workout worker is not rehydrated automatically.
- The worker exists only while a workout is active and is fully disposable

---

## Responsibilities and constraints

The `WorkoutWorker`:

- Is started explicitly when a workout begins and stopped explicitly when it ends
- May run in the background using platform-appropriate mechanisms (e.g. Android Foreground Service)
- Does **not** access Redux state, application services, or business logic
- Does **not** decide what should happen next in a workout
- Reacts to a stream of messages:
  - **Events**: Descriptive facts about what has occurred
  - **Commands**: Instructions to perform an action (e.g. finish workout)
- May emit observational events back to the main app (e.g. notification tapped)

---

## Android implementation note

On Android, the `WorkoutWorker` is implemented as a **ForegroundService**.
The service owns an **explicit, in-process message channel** used to distribute workout messages to platform-specific handlers.

This event channel:

- Exists entirely within the app process
- Provides ordered delivery
- Does not rely on OS-level routing (e.g. broadcasts or Intents)
- Is owned by the service itself

The service is responsible only for execution concerns (notifications, timers, system UI), not workout progression.

### Rest timer notification

While a rest timer runs, the persistent notification counts down. The short critical text (the status bar chip) and the subtext show the time left in the current part of the rest: first until the minimum rest is over, then until the maximum rest is over. The subtext adds the length of that part, as `0:45/1:00`. After the maximum rest, the subtext shows the time over it, as `+0:12`, and the critical text is empty. The `ProgressStyle` bar drains. `ProgressStyle` only fills from its start, so the bar's progress is the time left, and the minimum-rest segment and its point are at the far end, which empties first. The math is in `RestProgress` (`utils/RestProgress.kt`).

### Rest countdown pulses

While a rest timer runs, the worker vibrates once for 120 ms when 3 s, 2 s, and 1 s are left until the minimum rest is over, then for 400 ms when it is over. This lets the user feel the rest ending with the phone in a pocket. The pulses use alarm usage, so they also work in silent mode, where the "rest over" notification does not vibrate. They follow the same notifications setting as the rest notifications. On each 500 ms tick, `WorkoutUpdatedHandler` compares the time left with the value from the previous tick (`countdownSecondsCrossed` in `utils/RestCountdownVibrator.kt`). The handler keeps the previous value across workout updates while the rest end time stays the same. Thus an update during the countdown does not repeat or skip a pulse, and a restarted or resumed rest timer starts a new countdown. A paused rest timer sends no `restTimerInfo`, so it does not pulse.

---

## WorkoutMessages

### Conceptual model

`WorkoutMessages` can be either **events** or **commands**:

**Events** represent facts about what has occurred:

- `WorkoutStartedEvent`
- `WorkoutUpdatedEvent`
- `WorkoutEndedEvent`

**Commands** represent instructions for actions to perform:

- `FinishWorkoutCommand`

Messages are **bidirectional**:

- **App → Worker**: Events describing workout state changes (start, update, end)
- **Worker → App**: Commands requesting actions (e.g. finish workout from notification)

When the app broadcasts an event:

1. The event is immediately echoed back to all JS listeners (enabling local observation)
2. The event is forwarded to the native worker for platform-specific handling

---

### External representation (cross-platform)

For cross-platform communication (JS ↔ native, and parity with iOS), workout messages are serialized using **json** via the json schema we generate.
JSON schemas are generated from our types via (in ./app dir) `npm run json-schema`. This will generate schemas in the docs directory.
_Note this is currently broken, but may work in a future world_. Manually edit json schemas in the mean time.

---

### Translation boundary

A dedicated translation layer is responsible for converting between:

- Json Schema `WorkoutMessage` messages
- Internal domain `WorkoutMessage` objects (events and commands)

This translation layer:

- Lives on the JS side of the bridge
- Encapsulates versioning, defaults, and backward compatibility

The native bridge receives a json string directly and uses generated classes for parsing. The React Native module is responsible only for transport and service lifecycle.

---

## Message flow summary

### App → Worker (Events)

1. The main app broadcasts a `WorkoutMessage` (event)
2. The message is serialized using json
3. The broadcast immediately echoes the event back to all JS listeners
4. The native bridge forwards the message to the platform worker
5. The message is dispatched through the worker's in-process message channel
6. Platform-specific handlers react (update notifications, timers, etc.)

### Worker → App (Commands)

1. A platform-specific trigger occurs (e.g. notification action tapped)
2. The worker creates a command message (e.g. `FinishWorkoutCommand`)
3. The command is dispatched back to the app via the event bridge
4. The app handles the command (e.g. dispatches Redux action to finish workout)

---

## Lifecycle guarantees

- `WorkoutStartedEvent` explicitly starts the worker
- `WorkoutEndedEvent` explicitly stops the worker
- Messages outside an active workout lifecycle are ignored
