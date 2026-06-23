<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { PolicySummary } from '$lib/types';
  import PolicyList from '$lib/components/PolicyList.svelte';

  let policies = $state<PolicySummary[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  onMount(async () => {
    try {
      policies = await api.policies();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });
</script>

<section class="stack" style="padding: var(--space-lg) 0 var(--space-md)">
  <div class="row" style="justify-content: space-between; align-items: end">
    <div>
      <p class="kicker">Policy workspace</p>
      <h1>Signed policies you can prove.</h1>
      <p class="lede">
        Every <code>.aion</code> artifact carries its own version history, signatures, and audit
        trail. Open one to see its four-guarantee verification — structure, integrity, hash-chain,
        signatures — checked offline against the key registry.
      </p>
    </div>
    <a class="button" href="/new">New policy</a>
  </div>

  {#if loading}
    <div class="notice">Loading policies…</div>
  {:else if error}
    <div class="notice" style="color: var(--bad)">Could not load policies: {error}</div>
  {:else}
    <PolicyList {policies} />
  {/if}
</section>
