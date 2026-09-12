#!/bin/bash
# SH33: reproducible artifact for --renderframe-quad --renderframe-quad-loop <N> —
# SUSTAINABLE TEXTURED real-geometry rendering. After the single proof textured-quad
# frame, the engine's OWN geometry wrapper re-drives clear(cycling bg) -> textured
# coherent-renderer draw -> swap repeatedly, so a recording proves every frame is a
# FRESH textured render (the textured/mesh analog of SH25b's triangle-loop; closes the
# last "sustainable real-geometry" property for the textured path a real main-loop
# frame drive needs).
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb + ffmpeg + ffprobe.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh33-quad-loop.txt
MP4=/home/hermes-worker/runs/sh33-quad-loop.mp4
rm -f "$LOG" "$MP4"
N="${N:-6}"
CAP_SECS="${CAP_SECS:-4}"
timeout 60 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle --renderframe-quad \
  --renderframe-quad-loop "$N" --kicker 0x106863af8 > "$LOG" 2>&1 &
PID=$!
for i in $(seq 1 90); do
  if grep -q "quad-loop] iter 2 " "$LOG" 2>/dev/null; then break; fi
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
echo "=== sustainable TEXTURED-quad loop iterations (distinct bgs = fresh draws) ==="
grep -oE "quad-loop] iter [0-9]+ drew\+swap Ok\(0x1\) bg=\[[0-9., ]+\]" "$LOG"
echo "=== count distinct background colors (expect >=2 => animated/fresh) ==="
grep -oE "bg=\[[0-9., ]+\]" "$LOG" | sort -u | wc -l
echo "=== first textured-quad readback (proves textured draw intact in the same run) ==="
grep -E "renderframe-quad] readback" "$LOG" | tail -1