<script lang="ts">
  import DialogTemplate from '$lib/components/dialog-template.svelte';
  import Ripple from '$lib/components/ripple.svelte';
  import { buttonClasses, inputClasses } from '$lib/css-classes';
  import { isLoopbackServerUrl, type ConnectionCheck } from '@lifestats/ttu-sync';
  import { createEventDispatcher, onDestroy } from 'svelte';
  import { embedded, pendingText, problemText, statusText } from './status-text';
  import { startSync } from './sync';

  const dispatch = createEventDispatcher<{ close: void }>();
  const sync = startSync();
  const { status, serverUrl, openBook } = sync;

  const invalid: Record<string, string> = {
    not_http_url: 'Enter an http:// or https:// address.',
    has_credentials: 'Remove the user name and password from the address.',
    has_query: 'Remove the ?query from the address.',
    has_fragment: 'Remove the #fragment from the address.'
  };

  let draft = $serverUrl ?? '';
  let result = '';
  let now = Date.now();
  const ticker = setInterval(() => (now = Date.now()), 1000);
  onDestroy(() => clearInterval(ticker));

  $: summary = statusText($status, now);
  $: pending = pendingText($status);
  $: onThisPhone = embedded && isLoopbackServerUrl(draft);

  function describe(check: ConnectionCheck): string {
    if ('ok' in check) return invalid[check.reason];
    if (check.kind === 'ok') return 'Reachable. This is Lifestats with the reading API.';
    return problemText(check.problem);
  }

  async function test() {
    result = 'Testing…';
    result = describe(await sync.testConnection(draft));
  }

  async function save() {
    const saved = await sync.configure(draft);
    result = saved.ok
      ? saved.serverUrl
        ? `Saved ${saved.serverUrl}.`
        : 'Cleared. Reading stays on this device.'
      : invalid[saved.reason];
    if (saved.ok) draft = saved.serverUrl ?? '';
  }
</script>

<DialogTemplate>
  <svelte:fragment slot="header">Lifestats</svelte:fragment>
  <svelte:fragment slot="content">
    <div class="flex max-w-md flex-col gap-3 text-sm" data-lifestats-panel>
      <label class="flex flex-col gap-1">
        Server address
        <input
          type="url"
          class={inputClasses}
          placeholder={embedded ? 'http://laptop:3000' : 'http://localhost:3000'}
          bind:value={draft}
          data-lifestats-url
        />
      </label>
      {#if onThisPhone}
        <p class="text-amber-700" data-lifestats-hint>
          On this phone, localhost means the phone itself. Use the laptop's address, for example
          http://laptop:3000.
        </p>
      {/if}
      {#if result}
        <p data-lifestats-result>{result}</p>
      {/if}
      <p data-lifestats-summary>{summary.text}</p>
      {#if pending}
        <p data-lifestats-pending>{pending}</p>
      {/if}
      {#if $status.kind !== 'starting' && $status.pending.missingLegacyBooks.length}
        <p>
          Older books {$status.pending.missingLegacyBooks.join(', ')} are no longer in this library.
          Claim them with curl using a key from another device.
        </p>
      {/if}
      {#if $status.kind !== 'starting' && $status.lastDeliveredAt}
        <p>Last delivered {new Date($status.lastDeliveredAt).toLocaleString()}</p>
      {/if}
      {#if $openBook}
        <details>
          <summary>This book</summary>
          <p class="break-all">{$openBook.title}</p>
          <p class="break-all" data-lifestats-book-key>{$openBook.key}</p>
          <p>{$openBook.length.toLocaleString()} characters</p>
        </details>
      {/if}
    </div>
  </svelte:fragment>
  <svelte:fragment slot="footer">
    <button class={buttonClasses} on:click={test} data-lifestats-test>
      Test
      <Ripple />
    </button>
    <button class={buttonClasses} on:click={save} data-lifestats-save>
      Save
      <Ripple />
    </button>
    <button class={buttonClasses} on:click={() => sync.retryNow()} data-lifestats-retry>
      Retry now
      <Ripple />
    </button>
    <button class={buttonClasses} on:click={() => dispatch('close')}>
      Close
      <Ripple />
    </button>
  </svelte:fragment>
</DialogTemplate>
