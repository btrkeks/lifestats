<script lang="ts">
  import {
    faCloud,
    faCloudArrowUp,
    faTriangleExclamation
  } from '@fortawesome/free-solid-svg-icons';
  import { baseIconClasses } from '$lib/css-classes';
  import { dialogManager } from '$lib/data/dialog-manager';
  import { dummyFn } from '$lib/functions/utils';
  import Fa from 'svelte-fa';
  import LifestatsPanel from './lifestats-panel.svelte';
  import { statusText } from './status-text';
  import { startSync } from './sync';

  const { status } = startSync();

  $: summary = statusText($status, Date.now());
  $: icon =
    summary.tone === 'error'
      ? faTriangleExclamation
      : summary.tone === 'busy'
        ? faCloudArrowUp
        : faCloud;
  $: dot = { quiet: '', busy: '', warn: 'bg-amber-400', error: 'bg-red-500' }[summary.tone];

  function open() {
    dialogManager.dialogs$.next([{ component: LifestatsPanel }]);
  }
</script>

<div
  tabindex="0"
  role="button"
  title={summary.text}
  aria-label={summary.text}
  data-lifestats-status={$status.kind}
  class="relative {baseIconClasses}"
  on:click={open}
  on:keyup={dummyFn}
>
  <Fa {icon} />
  {#if dot}
    <span class="absolute right-2 top-2 h-2 w-2 rounded-full {dot}" />
  {/if}
</div>
