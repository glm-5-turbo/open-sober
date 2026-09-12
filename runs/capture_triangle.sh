#!/bin/bash
# SH25: reproducible artifact for --renderframe-triangle — the engine's OWN
# coherent geometry draw path renders a REAL visible triangle: fabricate a
# coherent renderer (real 1-primitive list + vertex-descriptor + stride tables +
# IBO), create real VBO/EBO + compile/link a real shader program (all through the
# JIT GLES bridge), then drive wrapper 0x5b35288 so primitive-setup 0x5b353d0 runs
# its REAL loop and the wrapper dispatches a real indexed glDrawElements. Capture
# the presented frame and verify it contains red triangle pixels.
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb + ffmpeg + ffprobe.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh25-triangle.txt
PNG=/home/hermes-worker/runs/sh25-triangle.png
RAW=/home/hermes-worker/runs/sh25-triangle.rgb
rm -f "$LOG" "$PNG" "$RAW"
timeout 90 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle \
  --kicker 0x106863af8 > "$LOG" 2>&1 &
PID=$!
for i in $(seq 1 120); do
  if grep -q "post-draw swap returned Ok" "$LOG" 2>/dev/null; then break; fi
  if ! kill -0 "$PID" 2>/dev/null; then break; fi
  sleep 0.5
done
sleep 2
DISPNUM=$(grep -oE "on :[0-9]+" "$LOG" | head -1 | tr -d 'on :')
echo "capturing display :${DISPNUM:-none} (post-draw)"
if [ -n "$DISPNUM" ]; then
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x720 -i ":$DISPNUM" \
    -frames:v 1 -f rawvideo -pix_fmt rgb24 - 2>/dev/null > "$RAW"
  echo "RAW_BYTES=$(stat -c%s "$RAW" 2>/dev/null || echo 0)"
  if command -v ffmpeg >/dev/null 2>&1; then
    ffmpeg -y -loglevel error -f rawvideo -pix_fmt rgb24 -s 1280x720 -i "$RAW" "$PNG"
  fi
fi
wait $PID
echo "EXIT=$?"
echo "=== coherent geometry draw reached through bridge? ==="
grep -E "triangle\] |hostcall@gl(DrawElements|BindBuffer|BufferData|VertexAttribPointer|UseProgram|ClearColor|Clear)" "$LOG" | grep -vE "^  \[t=|VECLD" | tail -25
echo "=== pixel analysis (expect red triangle on dark-blue background) ==="
if [ -s "$RAW" ]; then
  python3 -c "
import sys
d=open('$RAW','rb').read()
W,H=1280,720
from collections import Counter
c=Counter()
reds=0; total=0
for j in range(0,len(d)-2,3):
    r,g,b=d[j],d[j+1],d[j+2]
    c[(r,g,b)]+=1; total+=1
    if r>150 and g<90 and b<90: reds+=1
print('total px=%d  red-triangle px=%d (%.2f%%)'%(total,reds,100.0*reds/total))
for col,cnt in c.most_common(4):
    print('  RGB%d,%d,%d x%d'%(col[0],col[1],col[2],cnt))
"
fi