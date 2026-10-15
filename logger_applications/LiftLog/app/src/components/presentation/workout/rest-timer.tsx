import React, { useCallback, useEffect, useState } from 'react';
import { ColorChoice } from '@/hooks/useAppTheme';
import { RestWindow } from '@/models/session-models';
import { Duration, OffsetDateTime } from '@js-joda/core';
import { ViewStyle } from 'react-native';
import { impactAsync, ImpactFeedbackStyle } from 'expo-haptics';
import { RestTimerControls } from '@/components/presentation/workout/rest-timer-controls';
import { formatTimeSpan, TimerPane, TimerSegment } from '@/components/presentation/workout/timer-pane';
import { useTranslate } from '@tolgee/react';
import { useDerivedState } from '@/hooks/useDerivedState';
import { getRestTimerState, restBounds, RestPhase } from '@/components/presentation/workout/rest-timer-state';

interface RestTimerProps {
  rest: RestWindow;
  startTime: OffsetDateTime;
  pausedAt: OffsetDateTime | undefined;
  style?: ViewStyle;
  onRestart: () => void;
  onDismiss: () => void;
  onTogglePause: () => void;
}

const phaseColor: Record<RestPhase, ColorChoice> = {
  resting: 'onSurfaceVariant',
  ready: 'green',
  over: 'orange',
};

export default function RestTimer({
  rest,
  startTime,
  pausedAt,
  style,
  onRestart,
  onDismiss,
  onTogglePause,
}: RestTimerProps) {
  const { t } = useTranslate();
  const paused = pausedAt !== undefined;
  const [jiggled, setJiggled] = useDerivedState(startTime, () => [] as string[]);

  const getTimerState = useCallback(() => {
    const now = pausedAt ?? OffsetDateTime.now();
    return getRestTimerState(rest, Duration.between(startTime, now).toMillis());
  }, [startTime, pausedAt, rest]);

  const [timerState, setTimerState] = useState(getTimerState());
  const [jiggling, setJiggling] = useState(false);

  const triggerJiggle = useCallback(
    (milestone: string) => {
      if (jiggled.includes(milestone)) return;
      impactAsync(ImpactFeedbackStyle.Heavy).catch(console.log);
      setJiggling(true);
      setTimeout(() => setJiggling(false), 10);
      setJiggled((j) => [...j, milestone]);
    },
    [jiggled, setJiggled],
  );

  useEffect(() => {
    const timer = setInterval(() => {
      const state = getTimerState();
      setTimerState(state);
      if (state.phase !== 'resting') triggerJiggle('ready');
      if (state.phase === 'over') triggerJiggle('over');
    }, 200);
    return () => clearInterval(timer);
  }, [getTimerState, triggerJiggle]);

  const { phase } = timerState;
  const { windowStart, windowEnd } = restBounds(rest);
  const accent = phaseColor[phase];

  const status = paused
    ? t('rest_timer.status.paused')
    : phase === 'resting'
      ? t('rest_timer.status.resting')
      : phase === 'ready'
        ? t('rest_timer.status.ready')
        : t('rest_timer.status.over');

  // Laid out last-to-first so the bar drains from its end like the notification's, the rest first.
  const segments: TimerSegment[] = [
    ...(windowEnd !== undefined
      ? [
          {
            flex: windowEnd - windowStart,
            progress: timerState.windowLeft,
            color: phaseColor[phase],
          },
        ]
      : []),
    {
      flex: windowStart,
      progress: timerState.restLeft,
      color: phase === 'over' && windowEnd === undefined ? phaseColor['over'] : phaseColor['resting'],
    },
  ];

  return (
    <TimerPane
      testID="rest-timer"
      time={phase === 'over' ? `+${formatTimeSpan(timerState.timeMs)}` : formatTimeSpan(timerState.timeMs)}
      status={status}
      accent={accent}
      segments={segments}
      jiggling={jiggling}
      style={style}
      controls={
        <RestTimerControls
          paused={paused}
          onRestart={() => {
            onRestart();
            triggerJiggle('reset');
          }}
          onTogglePause={onTogglePause}
          onDismiss={onDismiss}
        />
      }
    />
  );
}
