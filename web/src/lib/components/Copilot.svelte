<script lang="ts">
  let { id }: { id: string } = $props();

  let prompt = $state('');
  let answer = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  // A drafted ruleset, if the answer contains a ```json fenced block (apply it in the editor).
  const draft = $derived.by(() => {
    const m = answer.match(/```json\s*([\s\S]*?)```/);
    return m ? m[1].trim() : null;
  });

  async function ask() {
    const q = prompt.trim();
    if (!q || busy) return;
    busy = true;
    error = null;
    answer = '';
    try {
      const res = await fetch('/api/copilot/stream', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ policy_id: id, surface: 'editor', prompt: q }),
      });
      if (!res.ok || !res.body) throw new Error(`copilot request failed (${res.status})`);
      const reader = res.body.getReader();
      const dec = new TextDecoder();
      let buf = '';
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        buf += dec.decode(value, { stream: true });
        let i: number;
        while ((i = buf.indexOf('\n\n')) >= 0) {
          const line = buf
            .slice(0, i)
            .split('\n')
            .find((l) => l.startsWith('data: '));
          buf = buf.slice(i + 2);
          if (!line) continue;
          const data = JSON.parse(line.slice(6));
          if (data.token) answer += data.token;
          else if (data.error) error = data.error;
          else if (data.disabled) error = data.message;
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function copyDraft() {
    if (draft) await navigator.clipboard.writeText(draft);
  }
</script>

<div class="stack" style="gap: var(--space-sm)">
  <div class="row" style="gap: var(--space-sm)">
    <input
      class="text"
      style="flex: 1 1 20rem"
      placeholder="Ask the copilot… e.g. add a rule to deny refunds over 1000"
      bind:value={prompt}
      onkeydown={(e) => e.key === 'Enter' && ask()}
    />
    <button class="button" onclick={ask} disabled={busy}>{busy ? 'Thinking…' : 'Ask'}</button>
  </div>
  <p class="copilot-hint">Claude sees this policy's current rules, verification, governance, and audit state. It advises and drafts — you apply and sign.</p>

  {#if error}<div class="notice" style="color: var(--bad)">{error}</div>{/if}
  {#if answer}<div class="copilot-answer">{answer}</div>{/if}
  {#if draft}
    <div class="row">
      <button class="button ghost" onclick={copyDraft}>Copy drafted rules</button>
      <span class="copilot-hint">Paste into “Edit &amp; commit” below, review the diff, and commit.</span>
    </div>
  {/if}
</div>
