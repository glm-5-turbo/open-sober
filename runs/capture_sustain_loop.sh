#!/bin/bash
# SH23: reproducible artifact for the --rendersustain <fps> lever — the engine's
# OWN frame-fn 0x105b32c00 -> swap recipe runs CONTINUOUSLY on a detached host
# thread (while StartApp's main loop idles on the main thread), cycling the clear
# color through a 5-color palette every frame, so a recording proves every frame
# is a fresh render rather than a static buffer.
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb + ffmpeg + ffprobe.
#
# Usage: FPS=2 CAP_SECS=8 bash runs/capture_sustain_loop.sh
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh23-sustain-loop.txt
MP4=/home/hermes-worker/runs/sh23-sustain-loop.mp4
rm -f "$LOG" "$MP4"
FPS="${FPS:-2}"
CAP_SECS="${CAP_SECS:-8}"
timeout 90 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --rendersustain "$FPS" \
  --kicker 0x106863af8 > "$LOG" 2>&1 &
PID=$!
for i in $(seq 1 90); do
  if grep -q "frame iteration 2" "$LOG"; then break; fi
  if ! kill -0 "$PID" 2>/dev/null; then break; fi
  sleep 0.5
done
DISPNUM=$(grep -oE "on :[0-9]+" "$LOG" | head -1 | tr -d 'on :')
echo "capturing display :${DISPNUM:-none}"
if [ -n "$DISPNUM" ]; then
  sleep 1
  ffmpeg -y -f x11grab -video_size 1280x720 -framerate 2 -i ":$DISPNUM" \
    -t "$CAP_SECS" "$MP4" 2>>"$LOG"
  echo "RECORDED=$?"
fi
wait $PID
echo "EXIT=$?"
# Count how many distinct clear colors the sustain loop went through.
echo "=== distinct engine-sustained render iterations ==="
grep -oE "frame iteration [0-9]+ color \[[0-9.]+" "$LOG" | head -20
echo "=== per-frame majority color in the recording (proves fresh frames) ==="
if command -v ffprobe >/dev/null 2>&1 && [ -s "$MP4" ]; then
  N=$(ffprobe -v error -select_streams v:0 -count_frames -show_entries stream=nb_read_frames -of default=nokey=1:noprint_wrappers=1 "$MP4")
  N=${N:-0}
  for i in $(seq 0 $((N>0?N-1:0))); do
    ffmpeg -loglevel error -y -i "$MP4" -vf "select=eq(n\,$i)" -frames:v 1 -f rawvideo -pix_fmt rgb24 - 2>/dev/null \
      | python3 -c "
import sys
d=sys.stdin.buffer.read()
if len(d) < 1280*720*3: print('frame $i: no data'); sys.exit()
from collections import Counter
c=Counter()
for j in range(0,len(d)-2,3*101):
    c[(d[j],d[j+1],d[j+2])]+=1
(col,cnt)=c.most_common(1)[0]
print('frame $i: RGB%d,%d,%d frac=(%.3f,%.3f,%.3f) x%d/%d'%(col[0],col[1],col[2],col[0]/255,col[1]/255,col[2]/255,cnt,len(d)//3))
"
  done
fi