<script lang="ts">
  import { booksDbUpgradeBlocked$ } from '$lib/data/database/books-db/factory';
</script>

{#if $booksDbUpgradeBlocked$}
  <div
    role="alert"
    class="writing-horizontal-tb fixed inset-x-0 top-0 z-50 bg-amber-200 p-3 text-center text-black"
  >
    Close other ッツ tabs to finish updating.
  </div>
{/if}
