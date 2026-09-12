#!/bin/bash
# SH34: capture the rendered NxN textured-grid mesh frame (x11grab) + readback log.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh34-grid.txt
PNG=/home/hermes-worker/runs/sh34-grid.png
rm -f "$LOG" "$PNG"
N="${N:-6}"
(timeout 55 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle \
  --renderframe-quad --renderframe-grid "$N" --kicker 0x106863af8 \
  > "$LOG" 2>&1) &
PID=$!
# Wait until the mesh cell readbacks print (mesh drawn + present). The graphed
# window keeps presenting; x11grab a frame shortly after the mesh draw lands.
for i in $(seq 1 100); do
  if grep -q "cell(3,3)@.*RGBA" "$LOG" 2>/dev/null; then break; fi
  if ! kill -0 "$PID" 2>/dev/null; then break; fi
  sleep 0.5
done
DISPNUM=$(grep -oE "on :[0-9]+" "$LOG" | head -1 | tr -d 'on :')
echo "capturing display :${DISPNUM:-none}"
if [ -n "$DISPNUM" ]; then
  sleep 1
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x720 -frames:v 1 -i ":$DISPNUM" "$PNG" 2>>"$LOG"
  echo "FRAME=$?"
fi
wait $PID
echo "EXIT=$?"
echo "=== mesh upload ==="; grep -E "renderframe-quad\] vbo=" "$LOG" | tail -1
echo "=== wrapper result ==="; grep -E "renderframe-quad\] geometry wrapper 0x5b35288 returned" "$LOG" | tail -1
echo "=== cell readback count ==="; grep -cE "cell\(.*RGBA" "$LOG"
echo "=== frame sample ==="; [ -f "$PNG" ] && ls -la "$PNG"
echo "=== selected cell readbacks (truth from glReadPixels) ==="; grep -E "cell\((0,0|1,1|3,3|5,5)\)" "$LOG"