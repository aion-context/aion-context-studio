<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { api, shortHex } from '$lib/api';
  import type { FileInfo, VerificationReport } from '$lib/types';
  import VerifyReport from '$lib/components/VerifyReport.svelte';
  import PolicyEditor from '$lib/components/PolicyEditor.svelte';
  import Governance from '$lib/components/Governance.svelte';
  import Simulate from '$lib/components/Simulate.svelte';
  import Audit from '$lib/components/Audit.svelte';
  import Copilot from '$lib/components/Copilot.svelte';

  const id = $page.params.id ?? '';
  let info = $state<FileInfo | null>(null);
  let report = $state<VerificationReport | null>(null);
  let rules = $state<string>('');
  let error = $state<string | null>(null);
  let loading = $state(true);

  function when(ns: number): string {
    return new Date(ns / 1_000_000).toISOString().replace('T', ' ').slice(0, 19) + 'Z';
  }

  async function load() {
    try {
      const [i, r, ru] = await Promise.all([api.info(id), api.verify(id), api.rules(id)]);
      info = i;
      report = r;
      rules = ru.rules;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);
</script>

<section class="stack" style="padding: var(--space-lg) 0 var(--space-md)">
  <div>
    <p class="kicker"><a href="/">workspace</a> / policy</p>
    <h1>{id}</h1>
  </div>

  {#if loading}
    <div class="notice">Loading…</div>
  {:else if error}
    <div class="notice" style="color: var(--bad)">{error}</div>
  {:else if info && report}
    <div class="panel" style="background: var(--surface)">
      <h2>Copilot</h2>
      <p class="lede faint" style="margin: var(--space-2xs) 0 var(--space-sm); font-size: 0.9rem">
        Claude, with this policy's full context — advises and drafts; you apply and sign.
      </p>
      <Copilot {id} />
    </div>

    <div class="row" style="align-items: stretch; gap: var(--space-md); flex-wrap: wrap">
      <div class="panel" style="flex: 1 1 18rem">
        <h2>Verification</h2>
        <p class="lede faint" style="margin: var(--space-2xs) 0 var(--space-sm); font-size: 0.9rem">
          Checked offline against the key registry.
        </p>
        <VerifyReport {report} />
      </div>
      <div class="panel" style="flex: 1 1 18rem">
        <h2>Governance</h2>
        <p class="lede faint" style="margin: var(--space-2xs) 0 var(--space-sm); font-size: 0.9rem">
          K-of-N approval; resets each version.
        </p>
        {#key info.current_version}
          <Governance {id} />
        {/key}
      </div>
      <div class="panel" style="flex: 2 1 26rem">
        <h2>Version history</h2>
        <table>
          <thead>
            <tr><th>v</th><th>author</th><th>when</th><th>message</th><th>rules hash</th></tr>
          </thead>
          <tbody>
            {#each info.versions as v}
              <tr>
                <td>{v.version_number}</td>
                <td>{v.author_id}</td>
                <td class="mono">{when(v.timestamp)}</td>
                <td>{v.message}</td>
                <td><span class="mono">{shortHex(v.rules_hash)}</span></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>

    <div class="panel">
      <h2>Simulate</h2>
      <p class="lede faint" style="margin: var(--space-2xs) 0 var(--space-sm); font-size: 0.9rem">
        Evaluate a proposed action against the current rules — the "check before it acts" loop.
      </p>
      <Simulate {id} />
    </div>

    <div class="panel">
      <h2>Audit &amp; export</h2>
      <p class="lede faint" style="margin: var(--space-2xs) 0 var(--space-sm); font-size: 0.9rem">
        The recorded operation history; compliance report and JSON/YAML/CSV export for auditors.
      </p>
      {#key info.current_version}
        <Audit {id} />
      {/key}
    </div>

    <div class="panel">
      <h2>Edit &amp; commit</h2>
      <p class="lede faint" style="margin: var(--space-2xs) 0 var(--space-sm); font-size: 0.9rem">
        Edit the rules, preview the diff against the current version, then commit a new signed version.
      </p>
      {#key info.current_version}
        <PolicyEditor {id} initialRules={rules} onCommitted={load} />
      {/key}
    </div>
  {/if}
</section>
