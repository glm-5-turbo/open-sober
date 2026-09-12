#!/bin/bash
# SH22: reproduce the engine's OWN frame-fn 0x105b32c00 presenting a real
# colored frame through the GLES bridge, and verify the presented pixels match
# the requested --renderframe-color (proves the clear path honors the color arg).
#
# Requires: cargo build -p arm64jit --example elfjit (done in this repo), the
# real libroblox.so at ~/.cache/open-sober/robbox/libroblox.so, Xvfb, ffmpeg.
#
# Usage: CC="0.40,0.20,0.95,1" bash runs/capture_engine_color_frame.sh
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh22-engine-frame-color.txt
PNG=/home/hermes-worker/runs/sh22-engine-frame.png
RAW=/home/hermes-worker/runs/sh22-engine-frame.rgb
rm -f "$LOG" "$PNG" "$RAW"
CC="${CC:-0.40,0.20,0.95,1}"
timeout 70 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-color "$CC" \
  --kicker 0x106863af8 > "$LOG" 2>&1 &
PID=$!
for i in $(seq 1 70); do
  if grep -q "post-frame swap returned" "$LOG"; then break; fi
  if ! kill -0 "$PID" 2>/dev/null; then break; fi
  sleep 0.5
done
DISPNUM=$(grep -oE "on :[0-9]+" "$LOG" | head -1 | tr -d 'on :')
echo "capturing display :${DISPNUM:-none}"
if [ -n "$DISPNUM" ]; then
  sleep 2
  ffmpeg -y -f x11grab -video_size 1280x720 -i ":$DISPNUM" -frames:v 1 "$PNG" 2>>"$LOG"
  echo "CAPPED=$?"
  ffmpeg -y -i "$PNG" -f rawvideo -pix_fmt rgb24 "$RAW" 2>>"$LOG"
fi
wait $PID
echo "EXIT=$?"
echo "requested color: $CC"
echo "=== majority pixel (file-order r,g,b) ==="
python3 - "$RAW" <<'PY'
import sys, os
p = sys.argv[1]
if not os.path.exists(p) or os.path.getsize(p) < 1280*720*3:
    print("no raw frame captured"); sys.exit(1)
d = open(p,'rb').read()
from collections import Counter
c = Counter()
for i in range(0, len(d)-2, 3*97):
    r,g,b = d[i], d[i+1], d[i+2]
    c[(r,g,b)] += 1
tot = 1280*720
black = sum(1 for i in range(0,len(d)-2,3) if d[i]==0 and d[i+1]==0 and d[i+2]==0)
for col,cnt in c.most_common(1):
    r,g,b = col
    print("MAJORITY RGB%d,%d,%d frac=(%.3f,%.3f,%.3f) x%d" % (r,g,b,r/255,g/255,b/255,cnt))
print("black=%.4f%% (%d/%d px)" % (100.0*black/tot, black, tot))
PY