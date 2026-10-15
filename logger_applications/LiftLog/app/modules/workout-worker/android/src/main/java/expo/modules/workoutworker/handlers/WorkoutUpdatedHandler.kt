package expo.modules.workoutworker.handlers


import android.annotation.SuppressLint
import android.app.Notification
import android.util.Log
import com.limajuice.liftlog.DistanceCardioTarget
import com.limajuice.liftlog.RecordedCardioExercise
import com.limajuice.liftlog.RecordedCardioExerciseSet
import com.limajuice.liftlog.RecordedWeightedExercise
import com.limajuice.liftlog.TimeCardioTarget
import com.limajuice.liftlog.Translations
import com.limajuice.liftlog.Weight
import com.limajuice.liftlog.WeightUnit
import com.limajuice.liftlog.WorkoutMessage
import com.limajuice.liftlog.WorkoutUpdatedEvent
import expo.modules.workoutworker.utils.RepeatingTimerAction
import expo.modules.workoutworker.utils.RestCountdownVibrator
import expo.modules.workoutworker.utils.RestProgress
import expo.modules.workoutworker.utils.RestWindow
import expo.modules.workoutworker.utils.WorkoutNotificationManager
import expo.modules.workoutworker.utils.countdownSecondsCrossed
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.time.Clock
import kotlin.time.Duration
import kotlin.time.DurationUnit.SECONDS
import kotlin.time.ExperimentalTime
import kotlin.time.Instant
import kotlin.time.toDuration


class WorkoutUpdatedHandler(
    private val notificationManager: WorkoutNotificationManager,
    private val restCountdownVibrator: RestCountdownVibrator,
) : WorkoutMessageHandler {
    override fun canHandle(event: WorkoutMessage): Boolean {
        return event.payload is WorkoutUpdatedEvent && event.appConfiguration.notificationsEnabled
    }

    val timer = RepeatingTimerAction(MainScope(), {})

    // Every workout update restarts the timer callback, so "have we announced this target yet" has to
    // outlive it - otherwise a set left running past its target re-announces on every update.
    private var announcedCardioTarget: Pair<Int, Int>? = null

    // Outlives the timer callback for the same reason: an update landing in the last seconds of rest
    // must neither repeat nor swallow a pulse. Keyed by when rest is over, which restarting or resuming
    // the rest timer moves, so either starts a fresh countdown.
    private var lastRestCountdown: Pair<Instant, Long>? = null

    override suspend fun handle(
        event: WorkoutMessage,
        dispatch: (type: String, event: WorkoutMessage) -> Unit
    ) {
        try {
            val workoutUpdatedEvent = event.payload as WorkoutUpdatedEvent

            when {
                workoutUpdatedEvent.restTimerInfo != null -> showRestTimerNotification(event.translations, workoutUpdatedEvent)
                workoutUpdatedEvent.cardioTimerInfo != null -> showCardioTimerNotification(event.translations, workoutUpdatedEvent)
                workoutUpdatedEvent.currentExerciseDetails != null -> showCurrentExerciseNotification(
                    event.translations,
                    workoutUpdatedEvent
                )

                else -> showFinishedNotification(event.translations, workoutUpdatedEvent)
            }
        } catch (e: Exception) {
            Log.e("WorkoutUpdatedHandler", "Failed to handle workout updated event", e)
        }
    }

    private fun showFinishedNotification(translations: Translations, event: WorkoutUpdatedEvent) {
        // We should not be in a timer anymore
        timer.stop()

        val messageTemplate: String =
            translations.workoutPersistentNotificationFinishedMessage
        val message = messageTemplate.replace(
            "\$WEIGHT$", formatWeight(event.totalWeightLifted, truncateDecimals = true)
        ).replace(
            "\$TIME$", formatDuration(Duration.parseIsoString(event.workoutDuration))
        )

        val notifBuilder = notificationManager.createWorkoutNotificationBuilder(promote = false)
            .setContentText(message)
        notificationManager.notifyPersistent(notifBuilder.build())
    }

    private fun showCurrentExerciseNotification(translations: Translations, event: WorkoutUpdatedEvent) {
        // We should not be in a timer anymore
        timer.stop()

        // The collapsed pill and the always-on-display outline only surface the title and the short
        // critical text, not the content text - so the exercise goes in the title and the upcoming set
        // in the critical slot, leaving the "start now" prompt for the expanded card.
        val exerciseMessage = getCurrentExerciseMessage(translations, event)
        val criticalText = getCurrentExerciseCriticalText(event)
        val notifBuilder = notificationManager.createWorkoutNotificationBuilder()
            .apply { if (exerciseMessage.isNotEmpty()) setContentTitle(exerciseMessage) }
            .apply { if (criticalText.isNotEmpty()) setShortCriticalText(criticalText) }
            .setContentText(translations.workoutPersistentNotificationStartNowMessage)
        notificationManager.notifyPersistent(notifBuilder.build())
    }

    @OptIn(ExperimentalTime::class)
    private fun showRestTimerNotification(
        translations: Translations,
        workoutUpdatedEvent: WorkoutUpdatedEvent,
    ) {

        val restTimerInfo = workoutUpdatedEvent.restTimerInfo ?:return
        fun getProgress(): Long {
            val timeStartSecs = restTimerInfo.startedAt.epochSeconds
            val now = Clock.System.now().epochSeconds
            return now - timeStartSecs
        }

        val currentExerciseMessage = getCurrentExerciseMessage(translations, workoutUpdatedEvent)
        var previousProgress = getProgress()
        timer.updateCallback {
            val timeStartSecs = restTimerInfo.startedAt.epochSeconds
            val timePartiallyEndSecs = restTimerInfo.partiallyEndAt.epochSeconds
            val timeEndSecs = restTimerInfo.endAt.epochSeconds
            val progress = getProgress()
            val nowInstant = Clock.System.now()
            val now = nowInstant.epochSeconds

            val restOverAt = restTimerInfo.partiallyEndAt
            val remainingMs = restOverAt.toEpochMilliseconds() - nowInstant.toEpochMilliseconds()
            val previousRemainingMs =
                lastRestCountdown?.takeIf { it.first == restOverAt }?.second ?: remainingMs
            // Several marks only pass in one tick after a stall; overlapping buzzes would merge into one.
            val crossed = countdownSecondsCrossed(previousRemainingMs, remainingMs)
            if (crossed.isNotEmpty()) {
                restCountdownVibrator.pulse(if (0 in crossed) 400L else 120L)
            }
            lastRestCountdown = restOverAt to remainingMs

            val partialProgressMax = timePartiallyEndSecs - timeStartSecs
            val fullProgressMax = timeEndSecs - timeStartSecs

            val restNotif: Notification? = when {
                partialProgressMax in (previousProgress + 1)..progress && partialProgressMax != 0L -> notificationManager.createRestNotificationBuilder()
                    .setContentTitle(translations.workoutPersistentNotificationMinRestOverMessage)
                    .build()

                fullProgressMax in (previousProgress + 1)..progress && fullProgressMax != 0L -> notificationManager.createRestNotificationBuilder()
                    .setContentTitle(translations.workoutPersistentNotificationMaxRestOverMessage)
                    .build()

                else -> null
            }
            if (restNotif != null) {
                notificationManager.notifyRest(restNotif)

                MainScope().launch {
                    delay(10_000)
                    notificationManager.clearRestNotification()
                }
            }
            @Suppress("AssignedValueIsNeverRead")
            previousProgress = progress

            val message = when {
                now < timePartiallyEndSecs -> translations.workoutPersistentNotificationRestBreakMessage
                now in timePartiallyEndSecs..timeEndSecs -> translations.workoutPersistentNotificationStartSoonMessage
                else -> translations.workoutPersistentNotificationStartNowMessage
            }
            val rest = RestProgress(progress, minRest = partialProgressMax, maxRest = fullProgressMax)
            val window = rest.window
            val criticalText =
                if (rest.windowLeft > 0) formatDuration(rest.windowLeft.toDuration(SECONDS)) else ""
            val subText = when (window) {
                RestWindow.DONE -> "+${formatDuration(rest.overtime.toDuration(SECONDS))}"
                else -> "${formatDuration(rest.windowLeft.toDuration(SECONDS))}/${
                    formatDuration(rest.windowLength.toDuration(SECONDS))
                }"
            }
            val notifBuilder =
                notificationManager.createWorkoutNotificationBuilder()
                    .apply { if (currentExerciseMessage.isNotEmpty()) setContentTitle(currentExerciseMessage) }
                    .setSmallIcon(window.icon)
                    .setContentText(message)
                    .setShortCriticalText(criticalText)
                    .setSubText(subText)
                    .setStyle(
                        notificationManager.timerProgressStyle(
                            progress = rest.barProgress,
                            max = fullProgressMax.toInt(),
                            partialThreshold = rest.barMinRestMark,
                        )
                    )

            notificationManager.notifyPersistent(notifBuilder.build())
        }
        timer.start()
    }


    @OptIn(ExperimentalTime::class)
    private fun showCardioTimerNotification(
        translations: Translations,
        workoutUpdatedEvent: WorkoutUpdatedEvent,
    ) {
        val cardioTimerInfo = workoutUpdatedEvent.cardioTimerInfo?:return
        val currentExerciseMessage = getCurrentExerciseMessage(translations, workoutUpdatedEvent)
        if(cardioTimerInfo.currentBlockStartTime == null){
            return
        }

        // Nothing knows when a distance target will be met, so only a time target can be counted down to.
        val target = getRunningCardioSet(workoutUpdatedEvent)?.blueprint?.target
        val targetSecs = (target as? TimeCardioTarget)?.let {
            Duration.parseIsoString(it.value).toLong(SECONDS)
        }

        val setKey = cardioTimerInfo.exerciseIndex.toInt() to cardioTimerInfo.setIndex.toInt()
        timer.updateCallback {
            val currentDuration = Duration.parseIsoString(cardioTimerInfo.currentDuration)
            val timeStartSecs =
                cardioTimerInfo.currentBlockStartTime.epochSeconds - currentDuration.toInt(SECONDS)
            val now = Clock.System.now().epochSeconds
            val elapsedSecs = now - timeStartSecs

            val reached = targetSecs != null && elapsedSecs >= targetSecs
            if (reached && announcedCardioTarget != setKey) {
                announcedCardioTarget = setKey
                notificationManager.notifyRest(
                    notificationManager.createRestNotificationBuilder()
                        .setContentTitle(translations.workoutPersistentNotificationCardioTargetReachedMessage)
                        .build()
                )
                MainScope().launch {
                    delay(10_000)
                    notificationManager.clearRestNotification()
                }
            }

            val timeMessage = when {
                targetSecs == null -> formatDuration(elapsedSecs.toDuration(SECONDS))
                reached -> "+${formatDuration((elapsedSecs - targetSecs).toDuration(SECONDS))}"
                else -> formatDuration((targetSecs - elapsedSecs).toDuration(SECONDS))
            }

            val notifBuilder =
                notificationManager.createWorkoutNotificationBuilder()
                    .apply { if (currentExerciseMessage.isNotEmpty()) setContentTitle(currentExerciseMessage) }
                    .setContentText(translations.workoutPersistentNotificationInProgressMessage)
                    .setShortCriticalText(timeMessage)
                    .setSubText(timeMessage)
                    .setStyle(
                        notificationManager.timerProgressStyle(
                            progress = elapsedSecs.toInt(),
                            max = (targetSecs ?: 0L).toInt(),
                            indeterminate = targetSecs == null || reached,
                        )
                    )

            notificationManager.notifyPersistent(notifBuilder.build())
        }
        timer.start()
    }

    private fun getRunningCardioSet(event: WorkoutUpdatedEvent): RecordedCardioExerciseSet? {
        val cardioTimerInfo = event.cardioTimerInfo ?: return null
        val exercise = event.workout.recordedExercises
            .getOrNull(cardioTimerInfo.exerciseIndex.toInt()) as? RecordedCardioExercise ?: return null
        return exercise.sets.getOrNull(cardioTimerInfo.setIndex.toInt())
    }

    @SuppressLint("DefaultLocale")
    private fun formatDuration(dur: Duration): String {
        return dur.toComponents { days: Long, hours: Int, minutes: Int, seconds: Int, _ ->
            when {
                days > 0 -> String.format("%d:%02d:%02d:%02d", days, hours, minutes, seconds)
                hours > 0 -> String.format("%d:%02d:%02d", hours, minutes, seconds)
                else -> String.format("%d:%02d", minutes, seconds)
            }
        }
    }

    private fun getCardioTarget(event: WorkoutUpdatedEvent): String {
        val set = getRunningCardioSet(event) ?: run {
            val currentExercise = event.currentExerciseDetails ?: return ""
            val exercise = currentExercise.exercise as? RecordedCardioExercise? ?: return ""
            exercise.sets.getOrNull(currentExercise.setIndex.toInt()) ?: return ""
        }
        return when (val cardioTarget = set.blueprint.target) {
            is TimeCardioTarget -> formatDuration(Duration.parseIsoString(cardioTarget.value))

            is DistanceCardioTarget -> "${cardioTarget.value.value} ${cardioTarget.value.unit}"
            else -> ""
        }
    }

    // Compact, glanceable text for the always-on-display / status-bar chip, which only has room for
    // a few characters: which set is next for a weighted exercise, or the target for a cardio set.
    private fun getCurrentExerciseCriticalText(event: WorkoutUpdatedEvent): String {
        return when (val currentExercise = event.currentExerciseDetails?.exercise) {
            is RecordedWeightedExercise -> setPositionOf(currentExercise)
            is RecordedCardioExercise -> getCardioTarget(event)
            else -> ""
        }
    }

    private fun getCurrentExerciseMessage(translations: Translations, event: WorkoutUpdatedEvent): String {

        val currentExercise =
            event.currentExerciseDetails?.exercise
        val messageTemplate = translations.workoutPersistentNotificationCurrentExerciseMessage

        // Deliberately null once every set is filled, so a finished exercise stops advertising a weight.
        val weightedExercise = event.currentExerciseDetails?.exercise as? RecordedWeightedExercise?
        val nextSet = weightedExercise?.potentialSets
            ?.indexOfFirst { it.set == null }?.takeIf { it >= 0 }
            ?.let { weightedExercise.potentialSets.getOrNull(it) }

        return when {
            event.currentExerciseDetails == null -> ""
            currentExercise is RecordedWeightedExercise -> {
                val upcoming = listOfNotNull(
                    setPositionOf(currentExercise).takeIf { it.isNotEmpty() }?.let { "Set $it" },
                    nextSet?.weight?.let { formatWeight(it) },
                ).joinToString(" · ")
                messageTemplate.replace(
                    "\$EXERCISE_DESCRIPTOR$",
                    if (upcoming.isEmpty()) currentExercise.blueprint.name
                    else "${currentExercise.blueprint.name} - $upcoming",
                )
            }

            currentExercise is RecordedCardioExercise-> messageTemplate.replace(
                "\$EXERCISE_DESCRIPTOR$", "${currentExercise.blueprint.name} - ${getCardioTarget(event)}"
            )

            else -> ""
        }
    }

    // The next unchecked set among all of this workout's sets, added ones included: "2/3". Empty once
    // every set is checked.
    private fun setPositionOf(exercise: RecordedWeightedExercise): String {
        val next = exercise.potentialSets.indexOfFirst { it.set == null }.takeIf { it >= 0 } ?: return ""
        return "${next + 1}/${exercise.potentialSets.size}"
    }

    private fun formatWeight(weight: Weight, truncateDecimals: Boolean = false): String {
        val weightValue = if (truncateDecimals) weight.value.toBigInteger().toString() else weight.value.toString()
        val shortUnit = when (weight.unit) {
            WeightUnit.kilograms -> "kg"
            WeightUnit.pounds -> "lbs"
            else -> "units"
        }
        return "$weightValue$shortUnit"
    }

    override fun onDestroy() {
        timer.destroy()
    }
}
