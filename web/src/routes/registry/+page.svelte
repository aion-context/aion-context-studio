<script lang="ts">
  import { onMount } from 'svelte';
  import { api, registryExportUrl } from '$lib/api';
  import type { AuthorView } from '$lib/types';

  let authors = $state<AuthorView[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let busy = $state(false);

  async function load() {
    try {
      authors = await api.registry();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function act(fn: () => Promise<unknown>) {
    busy = true;
    error = null;
    try {
      await fn();
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  const badge = (s: string) => (s === 'active' ? 'ok' : s === 'revoked' ? 'bad' : 'warn');

  onMount(load);
</script>

<section class="stack" style="padding: var(--space-lg) 0 var(--space-md)">
  <div class="row" style="justify-content: space-between; align-items: end">
    <div>
      <p class="kicker">Key registry</p>
      <h1>Authors &amp; key epochs.</h1>
      <p class="lede">
        The accreditation layer: who may sign, and which key is valid when. Rotation and revocation
        are authorized by each author's master key and take effect from the next version — existing
        signed versions keep verifying. Export the trusted JSON for an offline verifier to pin.
      </p>
    </div>
    <div class="row">
      <a class="button ghost" href={registryExportUrl} download>Export trusted JSON</a>
      <button class="button" onclick={() => act(api.registerAuthor)} disabled={busy}>Register author</button>
    </div>
  </div>

  {#if loading}
    <div class="notice">Loading registry…</div>
  {:else if error}
    <div class="notice" style="color: var(--bad)">{error}</div>
  {/if}

  {#each authors as a}
    <div class="panel">
      <div class="row" style="justify-content: space-between; align-items: baseline">
        <h2>author {a.author_id}</h2>
        {#if a.epochs.at(-1)?.status === 'active'}
          <div class="row">
            <button class="button ghost" onclick={() => act(() => api.rotateKey(a.author_id))} disabled={busy}>Rotate key</button>
            <button class="button danger" onclick={() => act(() => api.revokeKey(a.author_id, 'compromised'))} disabled={busy}>Revoke</button>
          </div>
        {:else}
          <span class="badge bad">no active key</span>
        {/if}
      </div>
      <div style="margin-top: var(--space-sm)">
        {#each a.epochs as e}
          <div class="epoch">
            <span class="num">epoch {e.epoch}</span>
            <span>
              <span class="key">{e.public_key.slice(0, 16)}…</span>
              <span class="meta"> · from v{e.created_at_version}{e.detail ? ` · ${e.detail}` : ''}</span>
            </span>
            <span class="badge {badge(e.status)}">{e.status}</span>
          </div>
        {/each}
      </div>
    </div>
  {/each}
</section>
