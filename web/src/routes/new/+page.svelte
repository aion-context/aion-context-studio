<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import RuleBuilder from '$lib/components/RuleBuilder.svelte';

  let id = $state('');
  let rules = $state('');
  let rulesValid = $state(false);
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

<section class="newp">
  <div class="newp-head">
    <p class="kicker"><a href="/">workspace</a> / new policy</p>
    <h1>Compose a policy</h1>
    <p class="lede">
      Author the rules as a sequence of decisions — <em>when</em> the conditions hold, <em>then</em>
      the decision applies, first match wins. On create, the studio writes a signed
      <code>.aion</code> genesis version.
    </p>
  </div>

  <div class="field">
    <label class="field-lab" for="pid">Policy id</label>
    <div class="field-affix" class:invalid={!!id && !idOk}>
      <input id="pid" class="text" placeholder="refund-authorization" bind:value={id} spellcheck="false" autocomplete="off" />
      <span class="affix">.aion</span>
    </div>
    <p class="field-hint">Lowercase letters, digits, and hyphens — this names the artifact.</p>
  </div>

  <div class="newp-rules">
    <div class="newp-rules-head">
      <h2>Rules</h2>
      <span class="faint">first match wins</span>
    </div>
    <RuleBuilder bind:json={rules} bind:valid={rulesValid} />
  </div>

  <div class="newp-actions">
    <button class="button" onclick={create} disabled={!idOk || !rulesValid || busy}>
      {busy ? 'Creating…' : 'Create policy'}
    </button>
    <a class="button ghost" href="/">Cancel</a>
    {#if !idOk}<span class="faint act-hint">Name the policy to continue.</span>{/if}
  </div>
  {#if error}<div class="notice" style="color: var(--bad)">{error}</div>{/if}
</section>
