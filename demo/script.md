# aion-context-studio — demo voice script

**One idea:** *Turn the rules that govern your agents into a signed artifact you can prove.*
**Audience:** buyers / decision-makers (CTO, head of platform, head of security/compliance).
**Covers:** full studio tour — compose → verify → govern → simulate → audit → copilot.
**Runtime:** ~2:40 · **Voice (ElevenLabs):** *Sarah — Mature, Reassuring, Confident*
(`EXAVITQu4vr4xnSDxMaL`), model **`eleven_v3`** (latest); key `~/.creds/eleven.env`.
**Pronunciation:** spoken text respells a few tokens (on-screen text unchanged):
`aion → "eye-on"`, `.aion → "dot eye-on"`, `K-of-N → "K of N"`, `CSV → "C S V"`,
`GDPR → "G D P R"`, `HIPAA → "HIPPA"`.

> Numbers, hashes, and ids are never read aloud — they stay on screen. The voice carries the
> meaning; the screen carries the proof.

---

## Storyboard

| # | Screen / on-screen action | Voiceover | ~sec |
|---|---|---|---|
| 1 | **Workspace** home — settle on the policy list + brand. | Software agents now act on our behalf — approving refunds, moving money, changing records. The rules that govern them usually live in a prompt, or buried in code: easy to change, impossible to prove. aion-context turns your rules into a signed, verifiable artifact — a .aion policy — with a full history of who changed what, and a signature on every version. This is the studio for building them. | 24 |
| 2 | **/new** — the rule builder; show rule 1 (≤ 50 → allow), the OTHERWISE connector, rule 2 (> 500 → deny); click Create. | You author a policy as a sequence of decisions. When these conditions hold, then this is the outcome — first match wins. Refunds under fifty are auto-approved; anything over five hundred is denied. Conditions are typed — numbers, text, or true-false — so a rule means exactly what it says. When you create the policy, the studio writes a signed genesis version. From here on, it's an artifact you can prove. | 28 |
| 3 | **Policy** — Verification panel; the four checks tick green; VALID. | Every policy is verified the same way — offline, with no middleman. Four guarantees, checked every time: the structure is well-formed, the contents match their hash, the version chain is intact, and every signature is authentic. All four pass. This version is valid — and anyone holding the public registry can confirm it independently. | 22 |
| 4 | **Policy** — Governance panel; the K-of-N progress. | High-stakes policies shouldn't change on one person's say-so. Governance requires K-of-N approval — here, two of three registered signers must sign off before a version is accepted. The studio tracks the threshold live, and every new version starts approval fresh. | 18 |
| 5 | **Policy** — Simulate panel; enter an action, see decision + trace. | Before a policy ever governs a real action, you can test it. Simulate proposes an action — an amount, a tier, a flag — and shows the decision the policy would return, with a trace of exactly which rule matched and why. It's the check-before-it-acts loop, made visible. | 20 |
| 6 | **Policy** — Audit & export; the timeline, then a compliance report. | Everything is on the record. The audit trail is the policy's operation history — who did what, and when. From here you can generate a compliance report — for SOX, HIPAA, or GDPR — or export the whole artifact as JSON, YAML, or CSV for an auditor. | 18 |
| 7 | **Policy** — Copilot panel; ask it to add a rule; the drafted JSON streams in. | And woven through all of it is a copilot. Claude sees this policy's real context — its current rules, its verification state, its governance and history — and helps you read, draft, and refine. Ask it to add a rule, and it returns one, in the right format, ready to apply. But the line is firm: Claude advises and drafts. A human applies, and a human signs. | 26 |
| 8 | **Custody** scene — file vault → OS keyring, with the real `import-keys` output. | By default, this is a single-operator demo, so the keys sit on disk. But where they live is pluggable. Build the desktop edition, and the same studio keeps its signing keys in the operating system's keyring instead — the Keychain, the Credential Manager, the Secret Service. One command migrates an existing workspace: it checks every key against the registry, then moves it in. Nothing sensitive stays on disk — the policies, the signatures, the verification, all unchanged. Only custody moves. | 23 |
| 9 | **Workspace** — settle back on the policy list. | That's the studio: author, govern, verify, simulate, and audit — every policy a signed, provable artifact, with a copilot at your side. Verifiable control, for the age of autonomous agents. | 14 |

**Running total:** ~3:13.

---

## Pure narration (per-scene files live in `demo/narration/scene-N.txt`, with respellings)

1. Software agents now act on our behalf — approving refunds, moving money, changing records. The rules that govern them usually live in a prompt, or buried in code: easy to change, impossible to prove. aion-context turns your rules into a signed, verifiable artifact — a .aion policy — with a full history of who changed what, and a signature on every version. This is the studio for building them.

2. You author a policy as a sequence of decisions. When these conditions hold, then this is the outcome — first match wins. Refunds under fifty are auto-approved; anything over five hundred is denied. Conditions are typed — numbers, text, or true-false — so a rule means exactly what it says. When you create the policy, the studio writes a signed genesis version. From here on, it's an artifact you can prove.

3. Every policy is verified the same way — offline, with no middleman. Four guarantees, checked every time: the structure is well-formed, the contents match their hash, the version chain is intact, and every signature is authentic. All four pass. This version is valid — and anyone holding the public registry can confirm it independently.

4. High-stakes policies shouldn't change on one person's say-so. Governance requires K-of-N approval — here, two of three registered signers must sign off before a version is accepted. The studio tracks the threshold live, and every new version starts approval fresh.

5. Before a policy ever governs a real action, you can test it. Simulate proposes an action — an amount, a tier, a flag — and shows the decision the policy would return, with a trace of exactly which rule matched and why. It's the check-before-it-acts loop, made visible.

6. Everything is on the record. The audit trail is the policy's operation history — who did what, and when. From here you can generate a compliance report — for SOX, HIPAA, or GDPR — or export the whole artifact as JSON, YAML, or CSV for an auditor.

7. And woven through all of it is a copilot. Claude sees this policy's real context — its current rules, its verification state, its governance and history — and helps you read, draft, and refine. Ask it to add a rule, and it returns one, in the right format, ready to apply. But the line is firm: Claude advises and drafts. A human applies, and a human signs.

8. By default, this is a single-operator demo, so the keys sit on disk. But where they live is pluggable. Build the desktop edition, and the same studio keeps its signing keys in the operating system's keyring instead — the Keychain, the Credential Manager, the Secret Service. One command migrates an existing workspace: it checks every key against the registry, then moves it in. Nothing sensitive stays on disk — the policies, the signatures, the verification, all unchanged. Only custody moves.

9. That's the studio: author, govern, verify, simulate, and audit — every policy a signed, provable artifact, with a copilot at your side. Verifiable control, for the age of autonomous agents.
