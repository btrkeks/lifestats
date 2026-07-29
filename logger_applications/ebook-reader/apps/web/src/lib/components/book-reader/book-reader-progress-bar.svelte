<script lang="ts">
  import { sectionList$ } from '$lib/components/book-reader/book-toc/book-toc';

  export let exploredCharCount: number;
  export let bookCharCount: number;

  let hovered = false;

  $: progress = bookCharCount > 0 ? (exploredCharCount / bookCharCount) * 100 : 0;
  $: clampedProgress = Math.min(100, progress);
  $: complete = progress >= 100;
  $: chapterMarks = ($sectionList$ || [])
    .filter((s) => s.startCharacter != null && bookCharCount > 0)
    .map((s) => (s.startCharacter! / bookCharCount) * 100);
</script>

{#if bookCharCount > 0}
  <div
    class="progress-bar"
    class:complete
    class:hovered
    on:mouseenter={() => (hovered = true)}
    on:mouseleave={() => (hovered = false)}
    role="progressbar"
    aria-valuenow={clampedProgress}
    aria-valuemin={0}
    aria-valuemax={100}
  >
    <div class="progress-fill" style:width="{clampedProgress}%" />
    {#each chapterMarks as pos}
      <div class="chapter-tick" style:left="{pos}%" />
    {/each}
    {#if hovered}
      <div
        class="tooltip"
        style:left="{clampedProgress}%"
      >
        {clampedProgress.toFixed(1)}%
      </div>
    {/if}
  </div>
{/if}

<style>
  .progress-bar {
    position: fixed;
    bottom: 2rem;
    left: 0;
    right: 0;
    height: 4px;
    z-index: 15;
    background: rgba(128, 128, 128, 0.15);
    border: 1px solid rgba(128, 128, 128, 0.2);
    border-radius: 2px;
    transition: height 0.2s ease;
    pointer-events: auto;
  }

  .progress-bar.hovered {
    height: 8px;
    border-color: rgba(128, 128, 128, 0.35);
  }

  .progress-fill {
    height: 100%;
    background: rgba(128, 128, 128, 0.5);
    transition: width 0.3s ease-out, opacity 0.2s ease;
  }

  .progress-bar.hovered .progress-fill {
    opacity: 1;
  }

  .progress-bar.complete .progress-fill {
    opacity: 1;
    animation: shimmer 0.6s ease-out;
  }

  .chapter-tick {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--font-color);
    opacity: 0.4;
  }

  .tooltip {
    position: absolute;
    bottom: 100%;
    transform: translateX(-50%);
    margin-bottom: 4px;
    padding: 2px 6px;
    font-size: 11px;
    line-height: 1.2;
    color: var(--font-color);
    background: var(--background-color);
    border: 1px solid currentColor;
    border-radius: 3px;
    white-space: nowrap;
    opacity: 0.8;
    pointer-events: none;
  }

  @keyframes shimmer {
    0% {
      opacity: 0.4;
    }
    50% {
      opacity: 1;
      filter: brightness(1.3);
    }
    100% {
      opacity: 1;
      filter: brightness(1);
    }
  }
</style>
