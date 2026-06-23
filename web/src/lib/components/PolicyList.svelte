<script lang="ts">
  import type { PolicySummary } from '$lib/types';

  let { policies }: { policies: PolicySummary[] } = $props();
</script>

<div class="stack" style="gap: var(--space-sm)">
  {#each policies as p}
    <a class="policy-row" href="/policy/{p.id}">
      <div>
        <div class="name">{p.id}</div>
        <div class="meta">
          <span>v{p.current_version} · {p.version_count} version{p.version_count === 1 ? '' : 's'}</span>
          <span class="mono">file {p.file_id}</span>
        </div>
      </div>
      <span class="badge {p.valid ? 'ok' : 'bad'}">{p.valid ? 'verified' : 'invalid'}</span>
    </a>
  {/each}
  {#if policies.length === 0}
    <div class="notice">No policies yet.</div>
  {/if}
</div>
