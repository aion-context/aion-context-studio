<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { MultiSigProgress } from '$lib/types';

  let { id }: { id: string } = $props();

  let p = $state<MultiSigProgress | null>(null);
  let busy = $state<number | null>(null);
  let error = $state<string | null>(null);

  const pct = $derived(p ? Math.min(100, (p.valid_count / Math.max(1, p.required)) * 100) : 0);

  async function load() {
    try {
      p = await api.multisig(id);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function approve(author: number) {
    busy = author;
    error = null;
    try {
      p = await api.approve(id, author);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = null;
    }
  }

  onMount(load);
</script>

{#if p}
  <div class="row" style="justify-content: space-between; align-items: baseline">
    <span class="faint">{p.threshold}-of-{p.signers.length} approval · version {p.version}</span>
    <span class="badge {p.threshold_met ? 'ok' : 'warn'}">{p.threshold_met ? 'approved' : 'pending'}</span>
  </div>

  <div class="progress {p.threshold_met ? 'met' : ''}" style="margin: var(--space-sm) 0">
    <span style="width: {pct}%"></span>
  </div>
  <p class="faint" style="font-size: 0.85rem; margin-bottom: var(--space-sm)">
    {p.valid_count} of {p.required} required approvals
  </p>

  <div>
    {#each p.signers as s}
      <div class="signer">
        <span class="who">author {s}</span>
        {#if p.approvers.includes(s)}
          <span class="approved">approved</span>
        {:else}
          <button class="button ghost" onclick={() => approve(s)} disabled={busy === s}>
            {busy === s ? 'Signing…' : 'Approve'}
          </button>
        {/if}
      </div>
    {/each}
  </div>
  {#if error}<div class="notice" style="color: var(--bad); margin-top: var(--space-sm)">{error}</div>{/if}
{:else if error}
  <div class="notice" style="color: var(--bad)">{error}</div>
{:else}
  <div class="notice">Loading governance…</div>
{/if}
