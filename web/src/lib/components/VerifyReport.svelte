<script lang="ts">
  import type { VerificationReport } from '$lib/types';

  let { report }: { report: VerificationReport } = $props();

  const checks = $derived([
    { label: 'Structure is well-formed', ok: report.structure_valid },
    { label: 'Integrity hash matches', ok: report.integrity_hash_valid },
    { label: 'Version hash-chain intact', ok: report.hash_chain_valid },
    { label: 'All signatures verify', ok: report.signatures_valid },
  ]);
</script>

<div class="stack" style="gap: var(--space-sm)">
  <div class="verdict {report.is_valid ? 'pass' : 'fail'}">
    {report.is_valid ? 'VERIFIED' : 'INVALID'}
  </div>
  <div>
    {#each checks as c}
      <div class="check {c.ok ? 'pass' : 'fail'}">
        <span class="mark">{c.ok ? '✓' : '✗'}</span>
        <span class="label">{c.label}</span>
      </div>
    {/each}
  </div>
  {#if report.errors.length > 0}
    <div class="notice" style="color: var(--bad)">
      {#each report.errors as e}<div class="mono">{e}</div>{/each}
    </div>
  {/if}
</div>
