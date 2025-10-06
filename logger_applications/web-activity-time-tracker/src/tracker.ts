import Browser from 'webextension-polyfill';
import { isValidPage } from './utils/valid-page';
import { extractHostname } from './utils/extract-hostname';
import { injectTabsRepositorySingleton } from './repository/inject-tabs-repository';
import { isInBlackList } from './functions/black-list';
import { BadgeColor, useBadge } from './functions/useBadge';
import { INTERVAL_SAVE_STORAGE_DEFAULT, StorageParams } from './storage/storage-params';
import { injectStorage } from './storage/inject-storage';
import { useDailyIntervals } from './functions/useDailyIntervals';
import { ActiveTab } from './utils/active-tab';
import { isLimitExceeded } from './functions/limit-list';
import { Tab } from './entity/tab';
import { useBlockPage } from './functions/useBlockPage';
import { convertSummaryTimeToBadgeString } from './utils/converter';
import { Settings } from './functions/settings';
import { useNotificationList } from './functions/useNotificationList';
import { NotificationType, useNotification } from './functions/useNotification';
import { NudgeManager } from './functions/NudgeManager';

const activeTabInstance = ActiveTab.getInstance();

interface CurrentObj {
  tab: Tab;
  activeDomain: string;
}

let currentObj: CurrentObj | null;
let nudgedTabIdForTracker: number | undefined = undefined;

export async function initTracker() {
  setInterval(trackTime, 1000);
  setInterval(saveTabs, INTERVAL_SAVE_STORAGE_DEFAULT);
}

async function trackTime() {
  const repo = await injectTabsRepositorySingleton();
  const window = await Browser.windows.getLastFocused({ populate: true });
  if (window.focused) {
    const activeTab = window.tabs?.find(t => t.active === true);
    if (isValidPage(activeTab)) {
      const activeDomain = extractHostname(activeTab!.url);

      if (await isInBlackList(activeDomain)) {
        if (activeTab?.id && nudgedTabIdForTracker === activeTab.id) { // If current tab was the one being nudged
          await NudgeManager.deactivateNudge(activeTab.id);
          nudgedTabIdForTracker = undefined;
        }
        NudgeManager.resetActiveNudgeStateForTabChange();

        if (await canChangeBadge()) {
          await useBadge({
            tabId: activeTab?.id,
            text: 'n/a',
            color: BadgeColor.green,
          });
        }
      } else {
        // Site is NOT on blacklist
        if (
          currentObj != null &&
          currentObj.activeDomain == activeDomain &&
          !isActiveTabWasChanged(activeDomain)
        ) {
          await mainTrackerWrapper(activeTab!, activeDomain, currentObj.tab);
          nudgedTabIdForTracker = activeTab!.id;
          return;
        }

        // New domain or first time tracking
        NudgeManager.resetActiveNudgeStateForTabChange();
        if (nudgedTabIdForTracker !== undefined && activeTab?.id !== nudgedTabIdForTracker) {
          await NudgeManager.deactivateNudge(nudgedTabIdForTracker);
          // nudgedTabIdForTracker will be updated by mainTrackerWrapper if successful
        }

        let tab = repo.getTab(activeDomain);
        if (tab == undefined) {
          tab = await repo.addTab(activeDomain);
        }
        if (tab != undefined) {
          await mainTrackerWrapper(activeTab!, activeDomain, tab);
          nudgedTabIdForTracker = activeTab!.id;
        } else {
          // Failed to get/add tab, ensure nudge is off for safety
          if (nudgedTabIdForTracker === activeTab!.id || nudgedTabIdForTracker !== null) {
            await NudgeManager.deactivateNudge(activeTab!.id!);
            nudgedTabIdForTracker = undefined;
          }
          NudgeManager.resetActiveNudgeStateForTabChange();
        }
      }
    } else { // Not a valid page
      await closeOpenInterval(); // This will call deactivateNudge
    }
  } else { // Window not focused
    await closeOpenInterval(); // This will call deactivateNudge
  }
}

async function closeOpenInterval() {
  // NudgeManager's deactivateNudge will use its internally stored nudgedTabId
  // if we don't pass a specific one, or if we do, it handles that.
  // We need to ensure that IF a nudge was active, it gets told to stop.
  // The NudgeManager.deactivateNudge(tabId) should be smart enough.
  // If we have a specific tab ID that is closing/losing focus:
  const lastActiveDomain = activeTabInstance.getActiveTabDomain();
  // Potentially, we might not have the tab ID here directly if activeTabInstance only stores domain.
  // This is where NudgeManager's internal state is useful.
  // If NudgeManager.deactivateNudge can take an optional tabId, and defaults to its own tracked one.

  // Simplest: NudgeManager.deactivateNudge() without args, it uses its internal nudgedTabId.
  // Or, pass the tabId if available and relevant.
  // Let's assume nudgedTabIdForTracker holds the ID of the tab that was last actively tracked.
  if (nudgedTabIdForTracker !== null) {
    await NudgeManager.deactivateNudge(nudgedTabIdForTracker); // Tell manager to stop nudge for this specific tab
    nudgedTabIdForTracker = undefined; // Clear our local tracker
  }
  NudgeManager.resetActiveNudgeStateForTabChange(); // Always reset this state

  (await useDailyIntervals()).closeInterval(lastActiveDomain);
  activeTabInstance.setActiveTab(null);
  currentObj = null;
}

async function mainTracker(
  state: Browser.Idle.IdleState,
  activeTab: Browser.Tabs.Tab,
  activeDomain: string,
  tab: Tab,
) {
  function isAudible() {
    return state === 'idle' && activeTab.audible;
  }

  currentObj = {
    tab: tab,
    activeDomain: activeDomain,
  };

  const isAudibleValue = isAudible();
  if (state === 'active' || isAudibleValue) {
    const limitExceeded = await isLimitExceeded(activeDomain, tab);
    if (limitExceeded.IsLimitExceeded) {
      const summaryCounter = tab.days.at(-1)!.counter;
      await useBlockPage(
        activeDomain,
        activeTab.url!,
        limitExceeded.LimitTime!,
        summaryCounter,
        activeTab.favIconUrl,
      );
      return;
    }

    if (isActiveTabWasChanged(activeDomain)) {
      tab.incCounter();
      (await useDailyIntervals()).closeInterval(activeTabInstance.getActiveTabDomain());
      activeTabInstance.setActiveTab(activeTab.url!);
      (await useDailyIntervals()).addInterval(activeTabInstance.getActiveTabDomain());
    }
    if (tab.favicon == '' && activeTab.favIconUrl != undefined)
      tab.setFavicon(activeTab.favIconUrl);

    if (await useNotificationList().isNeedToShowNotification(activeDomain, tab)) {
      const message = (await Settings.getInstance().getSetting(
        StorageParams.NOTIFICATION_MESSAGE,
      )) as string;
      const title = `${activeDomain} notification`;
      await useNotification(NotificationType.WebSiteNotification, title, message);
    }

    tab.incSummaryTime();

    // Nudge Activation/Update:
    // This site is NOT on the blacklist, so we want the nudge.
    // The time for the nudge is the total time spent on this site today.
    if (activeTab.id !== undefined) {
      await NudgeManager.activateOrUpdateNudge(activeTab.id, tab.days.at(-1)!.summary);
      nudgedTabIdForTracker = activeTab.id; // Keep track of which tab is being nudged
    }

    const viewInBadge = await Settings.getInstance().getSetting(StorageParams.VIEW_TIME_IN_BADGE);

    if (await canChangeBadge()) {
      if (viewInBadge)
        await useBadge({
          tabId: activeTab?.id,
          text: convertSummaryTimeToBadgeString(tab.days.at(-1)!.summary),
          color: BadgeColor.blue,
        });
      else
        await useBadge({
          tabId: activeTab?.id,
          text: null,
          color: BadgeColor.none,
        });
    }
  } else await closeOpenInterval();
}

async function mainTrackerWrapper(activeTab: Browser.Tabs.Tab, activeDomain: string, tab: Tab) {
  const inactivityInterval = (await Settings.getInstance().getSetting(
    StorageParams.INTERVAL_INACTIVITY,
  )) as number;
  const number = Number(inactivityInterval);
  const state = await Browser.idle.queryState(number);
  await mainTracker(state, activeTab!, activeDomain, tab);
}

function isActiveTabWasChanged(activeDomain: string) {
  return activeDomain != activeTabInstance.getActiveTabDomain();
}

async function saveTabs() {
  const storage = injectStorage();
  const repo = await injectTabsRepositorySingleton();
  const tabs = repo.getTabs();
  await storage.saveTabs(tabs);
}

async function canChangeBadge() {
  return !(await Settings.getInstance().getSetting(StorageParams.IS_POMODORO_ENABLED)) as boolean;
}
