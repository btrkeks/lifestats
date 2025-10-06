import Browser from 'webextension-polyfill';
import { NUDGE_MESSAGE_TYPE, NudgeMessagePayload } from '../types/messages'; // Adjust path
import { logger } from '../utils/logger'; // Assuming you have a logger

// This variable will keep track of the tab that currently has an active nudge.
// This is important to ensure we only try to deactivate nudges on tabs that had them.
let nudgedTabId: number | null = null;

// To prevent spamming messages if a tab is already nudged and receives activate again
let isNudgeActiveForCurrentTab = false;

export class NudgeManager {
  static async activateOrUpdateNudge(tabId: number, currentTimeSpent: number): Promise<void> {
    try {
      const message: NudgeMessagePayload = {
        action: isNudgeActiveForCurrentTab ? 'UPDATE_NUDGE_TIME' : 'ACTIVATE_NUDGE',
        currentTimeSpent: currentTimeSpent,
      };
      await Browser.tabs.sendMessage(tabId, message);
      nudgedTabId = tabId; // Store the tabId for which nudge is active
      isNudgeActiveForCurrentTab = true;
      // logger.log(`NudgeManager: Sent ${message.action} to tab ${tabId} with time ${currentTimeSpent}`);
    } catch (error) {
      // Common error: "Could not establish connection. Receiving end does not exist."
      // This happens if the content script isn't injected or active on that tab yet.
      // It's often benign on first load or for special pages.
      if (error instanceof Error && error.message.includes('Receiving end does not exist')) {
        // logger.warn(`NudgeManager: Content script not ready on tab ${tabId}. Error: ${error.message}`);
      } else {
        logger.error('NudgeManager: Error sending activate/update message:', error);
      }
    }
  }

  static async deactivateNudge(tabId: number): Promise<void> {
    // Only try to deactivate if this tabId was the one we were nudging
    if (nudgedTabId !== tabId && nudgedTabId !== null) {
      // logger.log(`NudgeManager: Request to deactivate nudge on tab ${tabId}, but active nudge was on ${nudgedTabId}. Ignoring.`);
      // If we want to be aggressive and try to clear any potential nudge on tabId:
      // pass // or proceed with sending message but don't clear nudgedTabId unless it matches
    }

    // If a specific tabId is provided, and it matches the one we're tracking, or if we just want to clear any nudge on it.
    // However, the primary use case is to clear the *currently tracked* nudgedTabId.
    // For simplicity, we'll assume `deactivateNudge` is called for the tab that was actively being nudged.
    if (nudgedTabId === null && !isNudgeActiveForCurrentTab) {
      // logger.log(`NudgeManager: No nudge active, no need to deactivate for tab ${tabId}.`);
      return;
    }


    try {
      // We must use the stored nudgedTabId if we're clearing a generic "current" nudge.
      // If tabId is explicitly passed and differs, it means a specific tab closed/changed.
      const targetTabId = nudgedTabId !== null ? nudgedTabId : tabId;

      const message: NudgeMessagePayload = { action: 'DEACTIVATE_NUDGE' };
      await Browser.tabs.sendMessage(targetTabId, message);
      // logger.log(`NudgeManager: Sent DEACTIVATE_NUDGE to tab ${targetTabId}`);
    } catch (error) {
      if (error instanceof Error && error.message.includes('Receiving end does not exist')) {
        // logger.warn(`NudgeManager: Content script not ready on tab ${tabId} for deactivation. Error: ${error.message}`);
      } else {
        logger.error('NudgeManager: Error sending deactivate message:', error);
      }
    } finally {
      // Reset state only if the deactivated tab was the one being tracked
      if (nudgedTabId === tabId || nudgedTabId !== null) { // Ensure we clear if it was the active one
        nudgedTabId = null;
        isNudgeActiveForCurrentTab = false;
      }
    }
  }

  /**
   * Call this when the active tab domain changes or focus is lost,
   * ensuring the state tracking `isNudgeActiveForCurrentTab` is reset
   * so the next activation on a new tab sends 'ACTIVATE_NUDGE' not 'UPDATE_NUDGE_TIME'.
   */
  static resetActiveNudgeStateForTabChange(): void {
    // logger.log("NudgeManager: Resetting active nudge state due to tab/focus change.");
    // nudgedTabId is typically cleared by deactivateNudge.
    // This specifically resets the flag that optimizes ACTIVATE vs UPDATE.
    isNudgeActiveForCurrentTab = false;
    // nudgedTabId = null; // Also clear this to be safe, deactivateNudge should handle it.
  }
}

// Example for future switching:
// enum NudgeType { BORDER, GRAYSCALE }
// let currentNudgeSetting = NudgeType.BORDER;
// In activateNudge:
// message.nudgeType = currentNudgeSetting; // Content script would then decide which component/effect