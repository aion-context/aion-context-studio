<script lang="ts">
  import type { DiffLine } from '$lib/types';

  let { lines }: { lines: DiffLine[] } = $props();
  const gutter = (t: string) => (t === 'add' ? '+' : t === 'del' ? '−' : ' ');
  const changed = $derived(lines.some((l) => l.tag !== 'same'));
</script>

{#if !changed}
  <div class="diff"><div class="diff-empty">No changes yet — edit the rules above to preview a diff.</div></div>
{:else}
  <div class="diff">
    {#each lines as l}
      <div class="dl {l.tag}"><span class="g">{gutter(l.tag)}</span><span>{l.text || ' '}</span></div>
    {/each}
  </div>
{/if}
