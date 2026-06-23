# aion-context-studio demo artifact

Created: 2026-06-23
Surface recorded: the live studio at `http://127.0.0.1:8787` (workspace · new-policy builder ·
policy detail: verification, governance, simulate, audit/export, copilot).

## Final artifact

`demo/aion-context-studio-demo.mp4` — narrated studio tour, ~2:56.
Published: https://youtu.be/go-fghIWHN4

## Source artifacts

| Artifact | Path |
|---|---|
| Voice script / storyboard | `demo/script.md` |
| Per-scene narration text | `demo/narration/scene-1..8.txt` |
| Per-scene narration audio (ElevenLabs) | `demo/narration/scene-1..8.mp3` |
| Narration generator | `demo/generate-narration.sh` |
| Concatenated narration | `demo/narration.mp3` |
| Scene durations (sync source) | `demo/durations.json` |
| Playwright recorder | `demo/record-demo.js` |
| Raw screen recording | `demo/playwright-video/*.webm` |
| Poster / thumbnail | `demo/demo-poster.jpg` · `demo/demo-thumbnail.jpg` |

## Narration

- Voice: ElevenLabs **Sarah — Mature, Reassuring, Confident** (`EXAVITQu4vr4xnSDxMaL`),
  model **`eleven_v3`** (latest). Key: `~/.creds/eleven.env`.
- 8 scenes generated separately and measured, so the screen action tracks each line.
- Pronunciation respellings in the spoken text (not the on-screen script): `aion → "eye-on"`,
  `.aion → "dot eye-on"`, `K-of-N → "K of N"`, `CSV → "C S V"`, `GDPR → "G D P R"`,
  `HIPAA → "HIPPA"`. Hashes and ids are never read aloud.

## Scenes (start → beat)

| # | t (s) | Surface | Beat |
|---|------:|---------|------|
| 1 | 0.0   | Workspace | Agents act on rules; aion-context makes the rules a signed, provable artifact |
| 2 | 28.0  | New-policy builder | Compose a policy as WHEN→THEN decisions; create a signed genesis |
| 3 | 56.1  | Verification | Four guarantees, offline: structure · integrity · hash-chain · signatures |
| 4 | 79.4  | Governance | K-of-N approval (2-of-3); each version starts approval fresh |
| 5 | 97.2  | Simulate | Propose an action → decision + per-rule trace (check before it acts) |
| 6 | 116.3 | Audit & export | Operation log; compliance report; JSON/YAML/CSV export |
| 7 | 134.4 | Copilot | Claude sees the policy's context, drafts a rule; advises — humans sign |
| 8 | 160.3 | Workspace | Verifiable control, for the age of autonomous agents |

## Verification

Final MP4 (ffprobe): MP4 (`+faststart`), 176.48 s, H.264 1440×900 @ 25 fps, AAC 192 kbps.
Narration track 176.69 s (model `eleven_v3`). Every scene recorded within ~0.1 s of its narration
target. The policy shown (`payments-policy`) is real: created live through the builder, then verified,
simulated (`amount_usd: 800 → deny`), and drafted against by the live Claude copilot — all on camera.

## Reproduce

```sh
cargo run -p aion-studio-api                 # studio on 127.0.0.1:8787
bash demo/generate-narration.sh              # ElevenLabs → narration/*.mp3 + durations.json + narration.mp3
# clean workspace so scenes 1/8 are tidy and scene 2's create succeeds:
find studio-data -type f -name 'payments-policy.*' -delete
NODE_PATH=<playwright> node demo/record-demo.js
ffmpeg -y -i demo/playwright-video/*.webm -i demo/narration.mp3 \
  -c:v libx264 -crf 20 -pix_fmt yuv420p -r 25 -c:a aac -b:a 192k \
  -shortest -movflags +faststart demo/aion-context-studio-demo.mp4
```
