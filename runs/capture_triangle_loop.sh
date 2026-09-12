#!/bin/bash
# SH25b: reproducible artifact for --renderframe-triangle-loop <N> — SUSTAINABLE
# real-geometry rendering. The engine's OWN geometry wrapper re-drives clear ->
# coherent-renderer draw -> swap repeatedly, cycling the clear color through a
# palette each frame, so a recording proves every frame is a FRESH real-geometry
# render (the geometry analog of SH23's --rendersustain).
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb + ffmpeg + ffprobe.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh25-triangle-loop.txt
MP4=/home/hermes-worker/runs/sh25-triangle-loop.mp4
rm -f "$LOG" "$MP4"
N="${N:-6}"
CAP_SECS="${CAP_SECS:-4}"
timeout 60 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle \
  --renderframe-triangle-loop "$N" --kicker 0x106863af8 > "$LOG" 2>&1 &
PID=$!
for i in $(seq 1 90); do
  if grep -q "triangle-loop] iter 2 " "$LOG" 2>/dev/null; then break; fi
  if ! kill -0 "$PID" 2>/dev/null; then break; fi
  sleep 0.5
done
DISPNUM=$(grep -oE "on :[0-9]+" "$LOG" | head -1 | tr -d 'on :')
echo "capturing display :${DISPNUM:-none}"
if [ -n "$DISPNUM" ]; then
  sleep 1
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x720 -framerate 2 -i ":$DISPNUM" \
    -t "$CAP_SECS" "$MP4" 2>>"$LOG"
  echo "RECORDED=$?"
fi
wait $PID
echo "EXIT=$?"
echo "=== sustainable real-geometry loop iterations (distinct bgs = fresh draws) ==="
grep -oE "triangle-loop] iter [0-9]+ drew\+swap Ok\(0x1\) bg=\[[0-9., ]+\]" "$LOG"
echo "=== count distinct background colors ==="
grep -oE "bg=\[[0-9., ]+\]" "$LOG" | sort -u | wc -l