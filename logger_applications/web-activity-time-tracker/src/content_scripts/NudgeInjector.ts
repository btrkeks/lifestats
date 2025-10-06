import { createApp, App as VueApp } from 'vue';
import Browser from 'webextension-polyfill';
import NudgeBorderComponent from '../components/NudgeBorderComponent.vue';
import { NUDGE_MESSAGE_TYPE, NudgeMessagePayload } from '../types/messages';

const NUDGE_ROOT_ID = 'my-extension-nudge-root';
let vueAppInstance: VueApp<Element> | null = null;
let nudgeRootElement: HTMLDivElement | null = null;

function mountNudgeComponent(initialTimeSpent: number): void {
  if (document.getElementById(NUDGE_ROOT_ID)) {
    // Already mounted, perhaps update if needed, or rely on UPDATE_NUDGE_TIME
    console.warn('Nudge component root already exists.');
    return;
  }

  nudgeRootElement = document.createElement('div');
  nudgeRootElement.id = NUDGE_ROOT_ID;
  document.body.appendChild(nudgeRootElement);

  vueAppInstance = createApp(NudgeBorderComponent, {
    currentTimeSpentSeconds: initialTimeSpent,
  });
  vueAppInstance.mount(nudgeRootElement);
  // console.log('Nudge component mounted with initial time:', initialTimeSpent);
}

function updateNudgeComponentTime(newTimeSpent: number): void {
  if (vueAppInstance && vueAppInstance._instance?.props) {
    // This is a common way to update props externally for Vue 3,
    // but for complex state, event bus or Pinia store injected into component is cleaner.
    // For a single prop, direct update is often fine.
    (vueAppInstance._instance.props as any).currentTimeSpentSeconds = newTimeSpent;

    // Force a re-render if necessary, though Vue's reactivity should handle it.
    // vueAppInstance._instance.update(); // or vueAppInstance.proxy?.$forceUpdate();
    // console.log('Nudge component time updated to:', newTimeSpent);
  } else if (!document.getElementById(NUDGE_ROOT_ID)) {
    // If component was somehow removed but we get an update, re-mount it
    mountNudgeComponent(newTimeSpent);
  }
}


function unmountNudgeComponent(): void {
  if (vueAppInstance) {
    vueAppInstance.unmount();
    vueAppInstance = null;
    // console.log('Nudge component unmounted.');
  }
  if (nudgeRootElement && nudgeRootElement.parentNode) {
    nudgeRootElement.parentNode.removeChild(nudgeRootElement);
    nudgeRootElement = null;
  }
  if (document.getElementById(NUDGE_ROOT_ID)) {
    // Fallback cleanup
    document.getElementById(NUDGE_ROOT_ID)?.remove();
  }
}

Browser.runtime.onMessage.addListener((message: NudgeMessagePayload, sender) => {
  // Optional: Check sender if needed, e.g., sender.id === Browser.runtime.id
  if (message.action) { // Checking if message is of NudgeMessagePayload structure
    switch (message.action) {
      case 'ACTIVATE_NUDGE':
        if (message.currentTimeSpent !== undefined) {
          // If already active, this will effectively update it.
          // Or, ensure it's mounted if not.
          if (!vueAppInstance || !document.getElementById(NUDGE_ROOT_ID)) {
            mountNudgeComponent(message.currentTimeSpent);
          } else {
            updateNudgeComponentTime(message.currentTimeSpent);
          }
        }
        break;
      case 'UPDATE_NUDGE_TIME':
        if (message.currentTimeSpent !== undefined) {
          updateNudgeComponentTime(message.currentTimeSpent);
        }
        break;
      case 'DEACTIVATE_NUDGE':
        unmountNudgeComponent();
        break;
    }
  }
  // Return true for async responses if you were to send one
  return false;
});

// Clean up if the script is re-injected or page unloads (though less reliable for SPA unload)
window.addEventListener('beforeunload', () => {
  unmountNudgeComponent();
});

// console.log('NudgeInjector content script loaded.');