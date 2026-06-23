<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';

  let id = $state('');
  let rules = $state('policy: my-policy\nrules:\n  - id: default\n    decision: allow\n');
  let error = $state<string | null>(null);
  let busy = $state(false);

  const idOk = $derived(/^[a-z0-9-]{1,64}$/.test(id));

  async function create() {
    busy = true;
    error = null;
    try {
      await api.create(id, rules);
      await goto(`/policy/${id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      busy = false;
    }
  }
</script>

<section class="stack" style="padding: var(--space-lg) 0 var(--space-md); max-width: 48rem">
  <div>
    <p class="kicker"><a href="/">workspace</a> / new</p>
    <h1>New policy</h1>
    <p class="lede">A new <code>.aion</code> artifact is created with a signed genesis version.</p>
  </div>

  <label class="stack" style="gap: var(--space-3xs)">
    <span class="faint" style="font-size: 0.85rem">Policy id <span class="mono">[a-z0-9-]</span></span>
    <input class="text" placeholder="refund-authorization" bind:value={id} />
  </label>

  <label class="stack" style="gap: var(--space-3xs)">
    <span class="faint" style="font-size: 0.85rem">Rules</span>
    <textarea class="rules" bind:value={rules} spellcheck="false"></textarea>
  </label>

  <div class="row">
    <button class="button" onclick={create} disabled={!idOk || busy}>
      {busy ? 'Creating…' : 'Create policy'}
    </button>
    <a class="button ghost" href="/">Cancel</a>
  </div>
  {#if error}<div class="notice" style="color: var(--bad)">{error}</div>{/if}
</section>
