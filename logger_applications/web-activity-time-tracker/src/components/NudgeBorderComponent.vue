<template>
  <div :style="borderStyle" class="progressive-nudge-border"></div>
</template>

<script lang="ts">
import { defineComponent, computed, ref, PropType } from 'vue';

const MAX_THICKNESS = 50; // px
const MAX_OPACITY = 0.85; // 0 to 1
const TIME_TO_MAX_EFFECT_MINUTES = 10; // Time to reach max thickness/opacity
const SECONDS_TO_MAX_EFFECT = TIME_TO_MAX_EFFECT_MINUTES * 60;

export default defineComponent({
  name: 'NudgeBorderComponent',
  props: {
    // This prop will be updated by the content script every second
    currentTimeSpentSeconds: {
      type: Number,
      required: true,
    },
  },
  setup(props) {
    const borderStyle = computed(() => {
      const progressRatio = Math.min(props.currentTimeSpentSeconds / SECONDS_TO_MAX_EFFECT, 1);

      const thickness = 10 + (MAX_THICKNESS - 10) * progressRatio; // Start at 10px
      const opacity = 0.2 + (MAX_OPACITY - 0.2) * progressRatio; // Start at 0.2 opacity

      // Color can also transition, e.g., from yellow to red
      // For simplicity, keeping it red but changing opacity
      const redIntensity = Math.floor(150 + 105 * progressRatio); // From a lighter red/orange to full red

      return {
        border: `${Math.round(thickness)}px solid rgba(${redIntensity}, 0, 0, ${opacity.toFixed(2)})`,
      };
    });

    return {
      borderStyle,
    };
  },
});
</script>

<style scoped>
.progressive-nudge-border {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 2147483647; /* Max z-index */
  pointer-events: none; /* Allow clicks to pass through */
  box-sizing: border-box;
}
</style>