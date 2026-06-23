<script lang="ts">
  import { untrack } from 'svelte';
  import { api } from '$lib/api';
  import type { DiffLine } from '$lib/types';
  import DiffView from './DiffView.svelte';

  let {
    id,
    initialRules,
    onCommitted,
  }: { id: string; initialRules: string; onCommitted: () => void } = $props();

  // Seed the editor once; the parent remounts this component (via {#key}) after each commit.
  let text = $state(untrack(() => initialRules));
  let message = $state('');
  let diff = $state<DiffLine[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const changed = $derived(diff.some((l) => l.tag !== 'same'));

  // Recompute the preview diff (current rules vs the edit) shortly after typing stops.
  $effect(() => {
    const proposed = text;
    const timer = setTimeout(async () => {
      try {
        diff = await api.diff(id, proposed);
      } catch {
        /* preview only; ignore transient errors */
      }
    }, 300);
    return () => clearTimeout(timer);
  });

  async function commit() {
    busy = true;
    error = null;
    try {
      await api.commit(id, text, message.trim() || 'Update rules');
      message = '';
      onCommitted();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="stack" style="gap: var(--space-sm)">
  <textarea class="rules" bind:value={text} spellcheck="false"></textarea>

  <div>
    <h3 style="margin-bottom: var(--space-2xs)">Preview — current → proposed</h3>
    <DiffView lines={diff} />
  </div>

  <div class="row" style="gap: var(--space-sm)">
    <input class="text" style="flex: 1 1 16rem" placeholder="Commit message (e.g. raise refund cap)" bind:value={message} />
    <button class="button" onclick={commit} disabled={!changed || busy}>
      {busy ? 'Committing…' : 'Commit version'}
    </button>
  </div>
  {#if error}<div class="notice" style="color: var(--bad)">{error}</div>{/if}
</div>
