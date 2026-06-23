<script lang="ts">
  import { onMount } from 'svelte';
  import { api, exportUrl } from '$lib/api';
  import type { AuditView } from '$lib/types';

  let { id }: { id: string } = $props();

  let view = $state<AuditView | null>(null);
  let report = $state<string | null>(null);
  let framework = $state('generic');
  let error = $state<string | null>(null);
  let busy = $state(false);

  const when = (ns: number) =>
    new Date(ns / 1_000_000).toISOString().replace('T', ' ').slice(0, 19) + 'Z';

  async function load() {
    try {
      view = await api.audit(id);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function showReport() {
    busy = true;
    error = null;
    try {
      report = await api.complianceReport(id, framework);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

<div class="stack" style="gap: var(--space-sm)">
  {#if view}
    <div>
      {#each view.entries as e}
        <div class="audit-row">
          <span class="act">{e.action}</span>
          <span class="meta">{when(e.timestamp)} · author {e.author_id}{e.detail ? ` · ${e.detail}` : ''}</span>
          <span class="key" title={e.hash}>{e.hash.slice(0, 12)}…</span>
        </div>
      {/each}
    </div>
  {/if}

  <div class="row" style="gap: var(--space-sm)">
    <span class="faint" style="font-size: 0.85rem">Export</span>
    <a class="button ghost" href={exportUrl(id, 'json')} download>JSON</a>
    <a class="button ghost" href={exportUrl(id, 'yaml')} download>YAML</a>
    <a class="button ghost" href={exportUrl(id, 'csv')} download>CSV</a>
  </div>

  <div class="row" style="gap: var(--space-sm)">
    <select class="text" bind:value={framework}>
      <option value="generic">Generic</option>
      <option value="sox">SOX</option>
      <option value="hipaa">HIPAA</option>
      <option value="gdpr">GDPR</option>
    </select>
    <button class="button" onclick={showReport} disabled={busy}>
      {busy ? 'Generating…' : 'Compliance report'}
    </button>
  </div>

  {#if error}<div class="notice" style="color: var(--bad)">{error}</div>{/if}
  {#if report}<pre class="report">{report}</pre>{/if}
</div>
