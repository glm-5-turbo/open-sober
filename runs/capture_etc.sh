#!/bin/bash
# SH27: reproducible artifact for --renderframe-triangle --renderframe-etc — the
# engine's OWN coherent geometry draw path renders a REAL COMPRESSED-ETC1 texture.
# A hand-crafted 8x8 ETC1 texture (4 solid 4x4 blocks, individual mode + modifier)
# is uploaded via glCompressedTexImage2D (GL_ETC1_RGB8_OES=0x8d64); the GLES bridge
# intercepts it, decodes ETC1->RGBA (texture-codec) and re-uploads via glTexImage2D.
# The textured fragment shader samples it; 3 on-triangle probes must read back
# distinct decoded colors.
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb + ffmpeg.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh27-etc.txt
PNG=/home/hermes-worker/runs/sh27-etc.png
RAW=/home/hermes-worker/runs/sh27-etc.rgb
rm -f "$LOG" "$PNG" "$RAW"
timeout 90 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle --renderframe-etc \
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
  ffmpeg -y -loglevel error -f rawvideo -pix_fmt rgb24 -s 1280x720 -i "$RAW" "$PNG"
fi
wait $PID
echo "EXIT=$?"
echo "=== compressed-ETC1 draw + readbacks (expect WHITE/GREEN/RED distinct) ==="
grep -E "renderframe-(tex|etc)|compile_status|readback|hostcall@glCompressedTexImage2D" "$LOG" | grep -vE "^  \[t=|VECLD" | tail -15
echo "=== pixel analysis (expect clear-blue bg + red/green/blue/white decoded-ETC1 quadrants) ==="
if [ -s "$RAW" ]; then
  python3 -c "
d=open('$RAW','rb').read()
from collections import Counter
c=Counter()
for j in range(0,len(d)-2,3):
    # 0/2-blended colors -> bucket: near-red, near-green, near-blue, white, bg
    r,g,b=d[j],d[j+1],d[j+2]
    if r>150 and g<90 and b<90: c['RED(255,2,2)']+=1
    elif g>150 and r<90 and b<90: c['GREEN(2,255,2)']+=1
    elif b>150 and r<90 and g<90: c['BLUE(2,2,255)']+=1
    elif r>150 and g>150 and b>150: c['WHITE']+=1
    elif r<40 and g<40 and b<110: c['BG(0,0,76)']+=1
    else: c['other']+=1
print('total px=%d'%(len(d)//3))
for k,v in c.most_common(): print('  %s x%d'%(k,v))
"
fi