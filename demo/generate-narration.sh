#!/usr/bin/env bash
# Generate per-scene narration via ElevenLabs (Sarah, eleven_v3), measure durations for sync,
# and concatenate to demo/narration.mp3. Key from ~/.creds/eleven.env.
set -euo pipefail
cd "$(dirname "$0")"
# shellcheck disable=SC1090
source ~/.creds/eleven.env

VOICE=EXAVITQu4vr4xnSDxMaL          # Sarah — Mature, Reassuring, Confident
MODEL="${ELEVEN_MODEL:-eleven_v3}"  # latest; override with ELEVEN_MODEL if needed
SCENES=(1 2 3 4 5 6 7 8 9)

declare -A DUR
for i in "${SCENES[@]}"; do
  body=$(jq -Rs --arg m "$MODEL" '{text: ., model_id: $m}' < "narration/scene-$i.txt")
  code=$(curl -s -w '%{http_code}' -o "narration/scene-$i.mp3" \
    -X POST "https://api.elevenlabs.io/v1/text-to-speech/${VOICE}?output_format=mp3_44100_128" \
    -H "xi-api-key: ${ELEVEN_LABS_API_KEY}" -H "Content-Type: application/json" \
    -d "$body")
  if [ "$code" != "200" ]; then
    echo "scene $i FAILED (HTTP $code): $(head -c 400 "narration/scene-$i.mp3")"; exit 1
  fi
  DUR[$i]=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "narration/scene-$i.mp3")
  echo "scene $i: ${DUR[$i]}s (model $MODEL)"
done

{ echo '{'; for i in "${SCENES[@]}"; do s=','; [ "$i" = "${SCENES[-1]}" ] && s=''; printf '  "%d": %s%s\n' "$i" "${DUR[$i]}" "$s"; done; echo '}'; } > durations.json

: > concat.txt
for i in "${SCENES[@]}"; do echo "file 'narration/scene-$i.mp3'" >> concat.txt; done
ffmpeg -y -f concat -safe 0 -i concat.txt -c copy narration.mp3 >/dev/null 2>&1

echo "TOTAL: $(ffprobe -v error -show_entries format=duration -of csv=p=0 narration.mp3)s"
