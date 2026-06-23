<script lang="ts">
  import { api } from '$lib/api';
  import type { Decision } from '$lib/types';

  let { id }: { id: string } = $props();

  let inputText = $state('{ "amount_usd": 75 }');
  let result = $state<Decision | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const tone = (d: string) => (d === 'allow' ? 'ok' : d === 'deny' ? 'bad' : 'warn');

  async function run() {
    busy = true;
    error = null;
    let parsed: Record<string, number>;
    try {
      parsed = JSON.parse(inputText);
    } catch {
      error = 'Action input is not valid JSON.';
      busy = false;
      return;
    }
    try {
      result = await api.simulate(id, parsed);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="stack" style="gap: var(--space-sm)">
  <label class="stack" style="gap: var(--space-3xs)">
    <span class="faint" style="font-size: 0.85rem">Action input (JSON)</span>
    <textarea class="rules" style="min-height: 5rem" bind:value={inputText} spellcheck="false"></textarea>
  </label>
  <div class="row">
    <button class="button" onclick={run} disabled={busy}>{busy ? 'Evaluating…' : 'Evaluate'}</button>
    {#if result}
      <span class="badge {tone(result.decision)}">{result.decision}</span>
      {#if result.matched_rule}
        <span class="faint" style="font-size: 0.85rem">via <span class="mono">{result.matched_rule}</span></span>
      {/if}
    {/if}
  </div>

  {#if error}<div class="notice" style="color: var(--bad)">{error}</div>{/if}

  {#if result}
    <div>
      {#each result.trace as t}
        <div class="trace-row {t.matched ? 'hit' : ''}">
          <span class="g">{t.matched ? '✓' : '·'}</span>
          <span class="mono">{t.rule_id}</span>
          <span class="faint">{t.note}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
