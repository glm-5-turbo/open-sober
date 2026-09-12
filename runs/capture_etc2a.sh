#!/bin/bash
# SH31: reproducible artifact for --renderframe-triangle --renderframe-quad
# --renderframe-etc2a — the engine's OWN coherent geometry draw path renders a REAL
# ETC2-RGBA8/EAC compressed texture (GL_COMPRESSED_RGBA8_ETC2_EAC=0x9278, the real
# Android RGBA-EAC format). The 8x8 texture = 4 x 16-byte blocks: each block's RGB is
# the SH29-proven ETC2 color (red/green/blue/white) and its EAC alpha sub-block is a
# distinct value (255/190/128/64). The fragment shader maps the DECODED ALPHA to RGB
# (gray-scale), so the 4 quadrant readbacks must read 4 distinct gray levels —
# robust proof decode_etc2_rgba8 ran (the EAC-alpha half, which the RGB-only 0x9274
# path cannot exercise).
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb + ffmpeg.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh31-etc2a.txt
PNG=/home/hermes-worker/runs/sh31-etc2a.png
RAW=/home/hermes-worker/runs/sh31-etc2a.rgb
rm -f "$LOG" "$PNG" "$RAW"
timeout 90 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle --renderframe-quad --renderframe-etc2a \
  --kicker 0x106863af8 > "$LOG" 2>&1 &
PID=$!
for i in $(seq 1 120); do
  if grep -q "renderframe-quad.*swap returned Ok" "$LOG" 2>/dev/null; then break; fi
  if ! kill -0 "$PID" 2>/dev/null; then break; fi
  sleep 0.5
done
sleep 2
DISPNUM=$(grep -oE "on :[0-9]+" "$LOG" | head -1 | tr -d 'on :')
echo "capturing display :${DISPNUM:-none} (post-quad-draw)"
if [ -n "$DISPNUM" ]; then
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x720 -i ":$DISPNUM" \
    -frames:v 1 -f rawvideo -pix_fmt rgb24 - 2>/dev/null > "$RAW"
  echo "RAW_BYTES=$(stat -c%s "$RAW" 2>/dev/null || echo 0)"
  ffmpeg -y -loglevel error -f rawvideo -pix_fmt rgb24 -s 1280x720 -i "$RAW" "$PNG"
fi
wait $PID
echo "EXIT=$?"
echo "=== ETC2-RGBA8/EAC quad + readbacks (expect 4 DISTINCT gray levels 255/190/128/64 = decoded EAC alphas) ==="
grep -E "etc2a|renderframe-quad.*readback|renderframe-quad 0x5b35288 returned|swap returned" "$LOG" | grep -vE "^  \[t=|VECLD" | tail -8
echo "=== pixel analysis (expect clear-blue bg + gray-scale quad interior) ==="
if [ -s "$RAW" ]; then
  python3 -c "
d=open('$RAW','rb').read()
from collections import Counter
c=Counter()
for j in range(0,len(d)-2,3):
    r,g,b=d[j],d[j+1],d[j+2]
    if r>240 and g>240 and b>240: c['GRAY~255']+=1
    elif r>185 and r<200 and g>185 and g<200 and b>185 and b<200: c['GRAY~190']+=1
    elif r>120 and r<140 and g>120 and g<140 and b>120 and b<140: c['GRAY~128']+=1
    elif r>55 and r<80 and g>55 and g<80 and b>55 and b<80: c['GRAY~64']+=1
    elif r<40 and g<40 and b<110: c['BG(0,0,76)']+=1
    else: c['other']+=1
print('total px=%d'%(len(d)//3))
for k,v in c.most_common(): print('  %s x%d'%(k,v))
"
fi