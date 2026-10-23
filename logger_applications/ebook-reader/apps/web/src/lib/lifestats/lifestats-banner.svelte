<script lang="ts">
  import { statusText } from './status-text';
  import { startSync } from './sync';

  const { status } = startSync();
</script>

{#if $status.kind === 'storage_failed'}
  <div
    role="alert"
    data-lifestats-banner
    class="writing-horizontal-tb fixed inset-x-0 bottom-0 z-50 bg-red-700 p-2 text-center text-sm text-white"
  >
    {statusText($status, Date.now()).text}
  </div>
{/if}
