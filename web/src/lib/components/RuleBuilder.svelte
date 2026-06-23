<script lang="ts">
  // Compose a policy as a legible sequence of decisions: each rule reads "WHEN <conditions> THEN
  // <decision>", first match wins. Compiles structured state to the default JSON rule format with
  // typed values, and exposes the JSON + a validity flag via bindable props so the page can gate
  // submission.
  type VType = 'number' | 'string' | 'boolean';
  type Cond = { field: string; op: string; vtype: VType; value: string };
  type Rule = { id: string; conds: Cond[]; decision: string };

  let { json = $bindable(''), valid = $bindable(false) }: { json?: string; valid?: boolean } =
    $props();

  const OP_SYM: Record<string, string> = { eq: '=', ne: '≠', lt: '<', le: '≤', gt: '>', ge: '≥' };
  const opsFor = (t: VType): string[] =>
    t === 'boolean' ? ['eq', 'ne'] : ['eq', 'ne', 'lt', 'le', 'gt', 'ge'];

  const STD = [
    { value: 'allow', label: 'Allow', tone: 'ok' },
    { value: 'allow_with_approval', label: 'Needs approval', tone: 'warn' },
    { value: 'deny', label: 'Deny', tone: 'bad' },
  ];
  const isCustom = (d: string) => !STD.some((s) => s.value === d);

  let rules = $state<Rule[]>([
    {
      id: 'auto-approve-small',
      conds: [{ field: 'amount_usd', op: 'le', vtype: 'number', value: '50' }],
      decision: 'allow',
    },
  ]);

  function conv(c: Cond): number | string | boolean {
    if (c.vtype === 'number') return Number(c.value || 0);
    if (c.vtype === 'boolean') return c.value === 'true';
    return c.value;
  }

  const compiled = $derived.by(() => {
    const issues: string[] = [];
    if (rules.length === 0) issues.push('Add at least one rule.');
    const seen = new Set<string>();
    rules.forEach((r, i) => {
      const label = r.id.trim() || `#${i + 1}`;
      if (!r.id.trim()) issues.push(`Rule ${i + 1}: give it an id.`);
      else if (seen.has(r.id.trim())) issues.push(`Duplicate rule id “${r.id.trim()}”.`);
      seen.add(r.id.trim());
      if (!r.decision.trim()) issues.push(`Rule ${label}: choose a decision.`);
      if (r.conds.length === 0) issues.push(`Rule ${label}: add at least one condition.`);
      r.conds.forEach((c, j) => {
        if (!c.field.trim()) issues.push(`Rule ${label}, condition ${j + 1}: name the field.`);
        if (c.vtype === 'number' && (c.value.trim() === '' || !Number.isFinite(Number(c.value))))
          issues.push(`Rule ${label}, condition ${j + 1}: value must be a finite number.`);
      });
    });
    const obj = {
      rules: rules.map((r) => ({
        id: r.id.trim(),
        when: Object.fromEntries(
          r.conds.filter((c) => c.field.trim()).map((c) => [c.field.trim(), { op: c.op, value: conv(c) }]),
        ),
        decision: r.decision.trim(),
      })),
    };
    return { json: JSON.stringify(obj, null, 2), issues };
  });

  $effect(() => {
    json = compiled.json;
    valid = compiled.issues.length === 0;
  });

  const blank = (): Cond => ({ field: '', op: 'le', vtype: 'number', value: '0' });
  const addRule = () => (rules = [...rules, { id: '', conds: [blank()], decision: 'allow' }]);
  const removeRule = (i: number) => (rules = rules.filter((_, k) => k !== i));
  const move = (i: number, d: -1 | 1) => {
    const j = i + d;
    if (j < 0 || j >= rules.length) return;
    const next = [...rules];
    [next[i], next[j]] = [next[j], next[i]];
    rules = next;
  };
  const addCond = (r: Rule) => (r.conds = [...r.conds, blank()]);
  const removeCond = (r: Rule, j: number) => (r.conds = r.conds.filter((_, k) => k !== j));
  function setType(c: Cond, t: VType) {
    c.vtype = t;
    if (!opsFor(t).includes(c.op)) c.op = 'eq';
    if (t === 'boolean' && c.value !== 'true' && c.value !== 'false') c.value = 'true';
  }
  function pickCustom(r: Rule) {
    if (!isCustom(r.decision)) r.decision = '';
  }
</script>

<div class="rb">
  {#each rules as rule, i}
    <div class="rb-rule">
      <div class="rb-head">
        <div class="rb-num">{String(i + 1).padStart(2, '0')}</div>
        <input class="rb-id" class:invalid={!rule.id.trim()} placeholder="rule-id" bind:value={rule.id} />
        <div class="rb-tools">
          <button class="iconbtn" onclick={() => move(i, -1)} disabled={i === 0} title="Move up" aria-label="move up">↑</button>
          <button class="iconbtn" onclick={() => move(i, 1)} disabled={i === rules.length - 1} title="Move down" aria-label="move down">↓</button>
          <button class="iconbtn danger" onclick={() => removeRule(i)} disabled={rules.length === 1} title="Remove rule" aria-label="remove rule">✕</button>
        </div>
      </div>

      <div class="rb-when">
        <span class="rb-lab">When</span>
        {#each rule.conds as cond, j}
          <div class="rb-cond">
            <input class="text f" class:invalid={!cond.field.trim()} placeholder="field" bind:value={cond.field} />
            <select class="text" value={cond.vtype} onchange={(e) => setType(cond, e.currentTarget.value as VType)} aria-label="value type">
              <option value="number">number</option>
              <option value="string">string</option>
              <option value="boolean">boolean</option>
            </select>
            <select class="text rb-op" bind:value={cond.op} aria-label="operator">
              {#each opsFor(cond.vtype) as op}<option value={op}>{OP_SYM[op]}</option>{/each}
            </select>
            {#if cond.vtype === 'boolean'}
              <select class="text v" bind:value={cond.value}><option value="true">true</option><option value="false">false</option></select>
            {:else if cond.vtype === 'number'}
              <input class="text v" inputmode="decimal" placeholder="0" class:invalid={cond.value.trim() === '' || !Number.isFinite(Number(cond.value))} bind:value={cond.value} />
            {:else}
              <input class="text v" placeholder="value" bind:value={cond.value} />
            {/if}
            <button class="iconbtn danger" onclick={() => removeCond(rule, j)} title="Remove condition" aria-label="remove condition">×</button>
          </div>
        {/each}
        <div class="rb-when-add"><button class="linkbtn" onclick={() => addCond(rule)}>+ add condition</button></div>
      </div>

      <div class="rb-then">
        <span class="rb-lab">Then</span>
        <div class="chips">
          {#each STD as d}
            <button class="chip {rule.decision === d.value ? 'sel ' + d.tone : ''}" onclick={() => (rule.decision = d.value)}>{d.label}</button>
          {/each}
          <button class="chip {isCustom(rule.decision) ? 'sel neutral' : ''}" onclick={() => pickCustom(rule)}>Custom…</button>
        </div>
        {#if isCustom(rule.decision)}
          <input class="text rb-custom" class:invalid={!rule.decision.trim()} placeholder="decision name" bind:value={rule.decision} />
        {/if}
      </div>
    </div>

    {#if i < rules.length - 1}<div class="rb-else">otherwise</div>{/if}
  {/each}

  <button class="rb-add" onclick={addRule}>+ Add rule</button>

  {#if compiled.issues.length > 0}
    <div class="rb-issues">
      <span class="h">{compiled.issues.length} thing{compiled.issues.length > 1 ? 's' : ''} to fix before this can be created</span>
      {#each compiled.issues as issue}<span>· {issue}</span>{/each}
    </div>
  {/if}

  <details class="rb-preview">
    <summary>Preview generated JSON</summary>
    <pre class="report" style="margin-top: var(--space-2xs)">{json}</pre>
  </details>
</div>
