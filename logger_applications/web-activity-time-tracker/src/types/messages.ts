export const NUDGE_MESSAGE_TYPE = 'NUDGE_CONTROL';

export interface NudgeMessagePayload {
  action: 'ACTIVATE_NUDGE' | 'DEACTIVATE_NUDGE' | 'UPDATE_NUDGE_TIME';
  currentTimeSpent?: number; // in seconds, for ACTIVATE_NUDGE and UPDATE_NUDGE_TIME
  // future ideas might add a 'nudgeType' field here
}