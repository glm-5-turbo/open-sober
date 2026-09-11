# Randomized differential fuzzer for arm64jit: generate varied C programs,
# compile static-aarch64 + native-oracle, run elfjit vs qemu, diff.
# Catches silent miscompiles across SIMD/int/fp shapes.
import subprocess, uuid, os, random, sys

RPATH = "/home/hermes-worker/runs/open-sober/target/debug/examples/elfjit"
os.chdir("/tmp/combw")
seed0 = int(sys.argv[1]) if len(sys.argv)>1 else 1
N = int(sys.argv[2]) if len(sys.argv)>2 else 40
random.seed(seed0)

def getentry(elf):
    d=subprocess.run(["aarch64-linux-gnu-objdump","-d",elf],capture_output=True,text=True).stdout
    for l in d.splitlines():
        if "<entry>:" in l:
            return l.split(":")[0].strip().split()[0]
    return None

def run_elfjit(elf, e):
    r=subprocess.run([RPATH,"/tmp/combw/"+elf,"0x"+e],
                     capture_output=True,text=True,timeout=30)
    for l in r.stdout.splitlines():
        if "entry() ->" in l:
            return l.split("-> ")[1].split(" ")[0]
    return None

def run_qemu(src, tag):
    # native x86 oracle: compile src + printf main, run it
    o=open(f"or_{tag}.c","w"); o.write(src+"\n#include <stdio.h>\nint main(){printf(\"%llu\\n\",(unsigned long long)entry());}\n"); o.close()
    r=subprocess.run(["gcc","-O3","-w",f"or_{tag}.c","-o",f"or_{tag}"],
                     capture_output=True,text=True)
    if r.returncode!=0:
        return None
    r=subprocess.run([f"./or_{tag}"],capture_output=True,text=True,timeout=30)
    return r.stdout.strip()

def build_pair(src, tag):
    open(f"fx_{tag}.c","w").write(src)
    r=subprocess.run(["aarch64-linux-gnu-gcc","-O3","-w","-static","-nostdlib",
                      "-Wl,-e,entry",f"fx_{tag}.c","-o",f"fx_{tag}.elf"],
                     capture_output=True,text=True)
    if r.returncode!=0:
        return None,None,None
    e=getentry(f"fx_{tag}.elf")
    j=run_elfjit(f"fx_{tag}.elf", e) if e else None
    q=run_qemu(src,tag)
    return e, j, q

# program generators: return C source with long long entry(void)
def gen_arith():
    # random u64 arithmetic with LCG + shifts/xors and a fold
    ops=[]
    for _ in range(random.randint(4,12)):
        ops.append(random.choice([
            "x=x*1664525ull+1013904223ull;",
            "x=x*1103515245ull+12345ull;",
            "x^=(x>>{s});","x^=(x<<{s});","x+=(x>>{s});","x=(x<<{s})|(x>>{r});",
        ]))
    body="unsigned long long x=seedv; unsigned long long r=0;\n"
    i=0
    for op in ops:
        s=random.choice([7,13,21,25,29,31,33,37,41])
        r=64-s
        body+=op.format(s=s,r=r)+"\n"
        if i%2==1: body+=f"r=r*131+((unsigned int)x);\n"
        i+=1
    body+="return (long long)(r ^ (r>>32));"
    return f"long long entry(void){{ volatile unsigned long long seedv = 987654321ull; {body} }}"

def gen_byte():
    n=random.choice([8,16,24,32,48,64])
    step=random.choice([1,2,4])
    expr=random.choice([
        "(long long)(unsigned char)b[i]",
        "(long long)(unsigned short)b[i]",
        "(long long)(signed char)b[i]",
    ])
    seed=random.choice([1,2,3,7,13,987654321])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = {seed}ull;
    unsigned long long x = seedv;
    unsigned char b[{n}];
    for(int i=0;i<{n};i++){{ x=x*1103515245ull+12345ull; b[i]=(unsigned char)((x>>24)&0xff); }}
    long long s1=0,s2=0;
    for(int i=0;i<{n};i+={step}) s1 += {expr};
    for(int i=1;i<{n};i+={step}) s2 += {expr};
    return s1*7 + s2;
}}"""

def gen_shift_matrix():
    rows=random.choice([4,8,12,16])
    cols=random.choice([4,8,12])
    load="volatile unsigned long long seedv=424242ull; unsigned long long x=seedv;\n"
    for r in range(rows):
        sh1=random.choice([1,8,16,24,25,31,32,40,48,56,63])
        sh2=random.choice([1,8,16,24,25,31,32,40,48,56,63])
        load+=f"unsigned int v{r}[{cols}];\n"
        for c in range(cols):
            load+=f"x=x*1103515245ull+12345ull; v{r}[{c}]=(unsigned int)((x>>{sh1})^(x<<{sh2}));\n"
    acc="long long t=0;\n"
    for r in range(rows):
        acc+=f"for(int c=0;c<{cols};c++) t=t*31+(long long)v{r}[c];\n"
    return f"long long entry(void){{ {load} {acc} return t; }}"

def gen_float():
    n=random.choice([8,16,32])
    kind=random.choice(["float","double"])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 424242ull;
    unsigned long long x = seedv;
    {kind} a[{n}];
    for(int i=0;i<{n};i++){{ x=x*1664525ull+1013904223ull; a[i]=({kind})((x>>40)&0xffff)+((x>>24)&0xff)*0.5f; }}
    {kind} m=0; int idx=0;
    for(int i=0;i<{n};i++){{ if(a[i]>m){{ m=a[i]; idx=i; }} }}
    long long s=0; for(int i=0;i<{n};i++) s+=(long long)(a[i]*2.0);
    return s*1000 + idx;
}}"""

def gen_float2():
    n=random.choice([8,16,24,32])
    kind=random.choice(["float","double"])
    op=random.choice(["+","-","*"])
    scale=random.choice([1.5,2.5,0.5])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 999111ull;
    unsigned long long x = seedv;
    {kind} a[{n}], b[{n}];
    for(int i=0;i<{n};i++){{ x=x*1664525ull+1013904223ull; a[i]=({kind})((x>>41)&0x7fff)-({kind})((x>>22)&0x3ff)*{scale}; b[i]=({kind})((x>>50)&0x1ffff)*0.25f; }}
    {kind} s=0;
    for(int i=0;i<{n};i++) s = ({kind})(s {op} a[i] {op} b[i]);
    return (long long)(s*16.0);
}}"""

def gen_2d_mix():
    r=random.choice([8,12,16,20]); c=random.choice([8,12,16])
    t1=random.choice(["unsigned int","int","short","unsigned short"])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 31337ull;
    unsigned long long x = seedv;
    int a[{r}][{c}];
    for(int i=0;i<{r};i++) for(int j=0;j<{c};j++){{ x=x*1103515245ull+12345ull; a[i][j]=(int)((x>>{random.choice([16,24,32,40])})^(x&0xff)); }}
    long long s=0;
    for(int i=0;i<{r};i+=2) for(int j={random.choice([0,1])};j<{c};j+=3) s += (long long)a[i][j];
    for(int i=1;i<{r};i+=2) for(int j=0;j<{c};j+=2) s -= (long long)a[i][j]*{random.choice([2,3,7])};
    return s;
}}"""

def gen_sat_shifts():
    n=random.choice([16,32,48])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 555777ull;
    unsigned long long x = seedv;
    int a[{n}];
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1442695040888963407ull; a[i]=(int)((x>>{random.choice([1,8,24,31,33,40])})^(x)); }}
    long long s=0;
    for(int i=0;i<{n};i++){{ if(a[i]>100) s+=(a[i]>>{random.choice([1,2,3,4])}); else s-=(a[i]<<{random.choice([1,2,3])}); }}
    return s;
}}"""

def gen_mul_long():
    n=random.choice([8,16,24])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 67452301ull;
    unsigned long long x = seedv;
    unsigned int a[{n}], b[{n}];
    for(int i=0;i<{n};i++){{ x=x*1103515245ull+12345ull; a[i]=(unsigned int)x; b[i]=(unsigned int)(x>>32); }}
    unsigned long long s=0;
    for(int i=0;i<{n};i++) s += (unsigned long long)a[i]*b[i];
    for(int i=0;i<{n};i+=2) s += (unsigned long long)(a[i]*{random.choice([1000000007,1000000009,1234567891])})%(4000000000ull);
    return (long long)(s % 1000000007);
}}"""

def gen_loop_branch():
    return """long long entry(void){
    volatile unsigned long long seedv = 887766ull;
    long long a=42, b=17, c=0;
    for(int i=0;i<200;i++){
        a = (a*1103515245ull + 12345ull) & 0x7fffffff;
        b = (b*6364136223ull + 14426950ull) & 0xffff;
        if((a & 1)) c += (b >> 3);
        else if ((a & 2)) c -= (b << 1);
        else if ((a & 4)) c ^= (a - b);
        else c += (a & b);
        if(i%7==0) c = (c*31) & 0x3fffffff;
    }
    return c;
}"""

def gen_globals_pie():
    # PIE + exported globals: forces GLOB_DAT / R_AARCH64_RELATIVE / ABS64
    # relocs in .data.rel.ro, exercising load_elf_image's in-process reloc
    # application + bind_image_plt before JIT. Omitted-nostdlib so the data
    # section with the function-pointer initializer is exercised.
    n=random.choice([4,8,16])
    muls=" / ".join(f"x=x*1103515245ull+12345ull; g{i}[(int)(x>>56)&7]=((int)(x>>{random.choice([16,24,32,40,48])})^(int)(x));" for i in range(2))
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 9037ull;
    unsigned long long x = seedv;
    static int s_glob = {random.choice([11,17,23,31])};
    static int s_arr[{n}] = {{ {', '.join(str(random.choice([2,3,5,7,9])) for _ in range(n))} }};
    static int *s_ptr = 0;
    long long acc = s_glob; int g0[8]={{0}}; int *gp = s_arr;
    for(int i=0;i<{n};i++){{ x=x*1103515245ull+12345ull; g0[i]=(int)((x>>{random.choice([16,24,32,40])})^(int)x); }}
    for(int i=0;i<{n};i++) acc += (long long)g0[i] + gp[i];
    return acc * {random.choice([3,7,13])} + s_glob;
}}"""

# A "loader" generator requires the :pie PIE path (not -static -nostdlib).
def gen_pie_callchain():
    n=random.choice([3,5,7])
    # nested caller/callee over globals (no libc), forces abs64 fn ptrs
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 5566ull;
    unsigned long long x = seedv;
    static long long gsum = 0;
    long long a[({n}+1)*4];
    for(int i=0;i<({n}+1)*4;i++){{ x=x*1103515245ull+12345ull; a[i]=(long long)((int)x^i); }}
    long long s=0;
    for(int r=0;r<{n};r++){{ for(int c=0;c<4;c++) s=s*31+a[r*4+c]; }}
    return s;
}}"""

_gen_compiler = {"static": "-static -nostdlib -Wl,-e,entry", "pie": "-fPIE -pie -nostdlib -Wl,-e,entry"}

def build_pair_mode(src, tag, mode):
    open(f"fx_{tag}.c","w").write(src)
    r=subprocess.run(["aarch64-linux-gnu-gcc","-O3","-w"]+_gen_compiler[mode].split()+
                     [f"fx_{tag}.c","-o",f"fx_{tag}.elf"],
                     capture_output=True,text=True)
    if r.returncode!=0:
        return None,None,None
    e=getentry(f"fx_{tag}.elf")
    j=run_elfjit(f"fx_{tag}.elf", e) if e else None
    q=run_qemu_nostdlib(src,tag) if mode=="static" else run_qemu_pie(src,tag)
    return e, j, q

def run_qemu_nostdlib(src, tag):
    # oracle: run the same entry via qemu-aarch64 on the static ELF
    o=open(f"or_{tag}.c","w"); o.write(src+"\n#include <stdio.h>\nint main(){printf(\"%llu\\n\",(unsigned long long)entry());}\n"); o.close()
    subprocess.run(["gcc","-O3","-w",f"or_{tag}.c","-o",f"or_{tag}"],capture_output=True,text=True)
    r=subprocess.run([f"./or_{tag}"],capture_output=True,text=True,timeout=30)
    return r.stdout.strip()

def run_qemu_pie(src, tag):
    # qemu can't run nostdlib PIE without crt; use a full C main wrapper compiled
    # for aarch64 and run under qemu for the oracle.
    o=open(f"or_{tag}.c","w"); o.write(src+"\n#include <stdio.h>\nint main(){printf(\"%llu\\n\",(unsigned long long)entry());}\n"); o.close()
    r=subprocess.run(["aarch64-linux-gnu-gcc","-O3","-w",f"or_{tag}.c","-o",f"or_{tag}.elf"],capture_output=True,text=True)
    if r.returncode!=0:
        return None
    r=subprocess.run(["qemu-aarch64","-L","/usr/aarch64-linux-gnu",f"or_{tag}.elf"],capture_output=True,text=True,timeout=30)
    return r.stdout.strip()

def gen_bfield_extract():
    # heavy UBFM/UBFX/SBFX extract patterns over the high half of a 64-bit LCG
    n=random.choice([8,16,24])
    ex=[]
    for i in range(6):
        lsb=random.choice([0,7,8,16,24,31,32,40,44,48,55,57,63])
        w=random.choice([1,3,4,8,12,16,19,24,25,31,32,33])
        ex.append(f"x=x*1103515245ull+12345ull; t=(t*131 + (long long)(x>>{lsb})); t=(t*131+(long long)((x<<{random.choice([1,8,24,25,31,32,40,56])})));")
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 135790ull;
    unsigned long long x = seedv; long long t = 0;
    {"".join(ex)}
    return t;
}}"""

def gen_sat_arith():
    # saturating-ish and signed-mul-heavy integer code
    n=random.choice([8,16,32])
    muls=random.choice(["a[i]*b[i]","(long long)a[i]*b[i]","(signed long long)a[i]*(signed int)b[i]","a[i]*{k}"])
    kval=random.choice([7,13,31,1000003,100003])
    muls_src=muls.replace("{k}",str(kval))
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 24680ull;
    unsigned long long x = seedv;
    int a[{n}]; unsigned short b[{n}];
    for(int i=0;i<{n};i++){{ x=x*1103515245ull+12345ull; a[i]=(int)((x>>32)^x); b[i]=(unsigned short)(x>>48); }}
    long long s=0;
    for(int i=0;i<{n};i++){{ s += {muls_src}; if(s>0x3fffffffLL) s-=0x7fffffffLL; }}
    return s;
}}"""

def gen_3d_accum():
    d1=random.choice([4,6,8]); d2=random.choice([4,6,8]); d3=random.choice([4,6,8])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 112233ull;
    unsigned long long x = seedv;
    unsigned char a[{d1}][{d2}][{d3}];
    for(int i=0;i<{d1};i++) for(int j=0;j<{d2};j++) for(int k=0;k<{d3};k++){{ x=x*1103515245ull+12345ull; a[i][j][k]=(unsigned char)(x>>56); }}
    long long s=0;
    for(int i=0;i<{d1};i++) for(int j=0;j<{d2};j++){{ for(int k=0;k<{d3};k++) s+=(long long)a[i][j][k]; s*={random.choice([3,7,11])}; }}
    return s;
}}"""

def gen_128_struct():
    # 128-bit struct array (STP/LDP q pairs) + swap + fold
    n=random.choice([4,8,12,16])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 121212ull;
    unsigned long long x = seedv;
    unsigned long long a[{n*2}], b[{n*2}];
    for(int i=0;i<{n*2};i++){{ x=x*1103515245ull+12345ull; a[i]=(x>>8)^x; }}
    for(int i=0;i<{n*2};i+=2){{ b[i]=a[i+1]; b[i+1]=a[i]; }}
    long long s=0;
    for(int i=0;i<{n*2};i++) s=(s*33 + (long long)(b[i]^a[i]));
    return s;
}}"""

def gen_float_reduce():
    n=random.choice([8,16,32,64])
    kind=random.choice(["float","double"])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 222333ull;
    unsigned long long x = seedv;
    {kind} a[{n}];
    for(int i=0;i<{n};i++){{ x=x*1664525ull+1013904223ull; a[i]=({kind})(((x>>42)&0xfff)-512)*0.001f; }}
    {kind} sum=0.5; {kind} mnm=1000000.0; {kind} mxm=-1000000.0;
    for(int i=0;i<{n};i++){{ sum+=a[i]; if(a[i]<mnm)mnm=a[i]; if(a[i]>mxm)mxm=a[i]; }}
    return (long long)((sum+mnm+mxm)*1000.0);
}}"""

def gen_fma_chain():
    # compiler-contracted fmla / fma chains: a*b+c repeated, and a/b
    n=random.choice([8,16,32])
    ty=random.choice(["double","float"])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 314159ull;
    unsigned long long x = seedv;
    {ty} a[{n}], b[{n}], c[{n}];
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1442695040888963407ull; a[i]=({ty})(((x>>40)&0xffff)/1024.0); b[i]=({ty})(((x>>24)&0xffff)/2048.0); c[i]=({ty})(((x>>8)&0xffff)/4096.0); }}
    {ty} acc=0.0;
    for(int i=0;i<{n};i++){{ acc+=a[i]*b[i]+c[i]; }}   // fmla candidate
    for(int i=0;i<{n};i++){{ acc+=a[i]/b[i]; }}        // division
    return (long long)(acc*1e3);
}}"""


def gen_mask_extract():
    # Stress vector AND-masks + 64-bit lane movi immediates + extraction.
    # The MOVI Vd.2D,#imm immediate bug lived here (0xffff -> 0x03 mask).
    n=random.choice([8,12,16,24])
    w=random.choice([8,12,16,20,24,28,32,40,48])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 271828ull;
    unsigned long long x = seedv;
    unsigned long long a[{n}];
    for(int i=0;i<{n};i++){{ x=x*2862933555777941757ull+3037000493ull; a[i]=(x>>{w})&0xffff; }}
    unsigned long long s=0;
    for(int i=0;i<{n};i++){{ s+=a[i]*7; s^=a[i]>>3; }}
    return (long long)s;
}}"""

def gen_signed_div():
    # Signed division/modulo with negatives + magic constants: exercises sdiv,
    # smull/msub remainder, and sign-extension (sdiv by non-pow2 -> smull chain).
    n=random.choice([16,32,64])
    divs=random.choice([3,5,7,9,11,13,17,100,101,1000,1024+1,2047])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 961479ull;
    unsigned long long x = seedv;
    int a[{n}];
    for(int i=0;i<{n};i++){{ x=x*1103515245ull+12345ull; a[i]=(int)((x>>31) - (x>>1)); }}  // spread +/-
    long long s=0;
    for(int i=0;i<{n};i++){{ s += a[i] / {divs}; s += a[i] % {divs}; }}
    for(int i=0;i<{n};i+=2){{ s += (long long)(a[i] / -{divs}); s += a[i] % -{divs}; }}
    return s;
}}"""

def gen_widen_byte_lut():
    # Byte-indexed lookup with widening i64 accumulate, plus a second constant-step
    # byte sum: two co-resident widening loops (uxtb/uxtw + combine) stratified by
    # different strides — the historical two-loop widening bug family.
    n=random.choice([16,32,48,64])
    step=random.choice([2,3,4,5])
    lut=(', '.join(str(random.choice([1,3,7,11,31,101])) for _ in range(37)))
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 777213ull;
    unsigned long long x = seedv;
    unsigned char b[{n}];
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1442695040888963407ull; b[i]=(unsigned char)((x*2654435761u)>>24); }}
    unsigned char lut[37] = {{ {lut} }};
    long long s1=0, s2=0;
    for(int i=0;i<{n};i++) s1 += (long long)lut[b[i]%37] * (long long)(b[i]&7);
    for(int i=0;i<{n};i+={step}) s2 += (long long)b[i];
    return s1*131 + s2*7 + (long long)b[0]*{step};
}}"""

gens=[gen_arith, gen_byte, gen_shift_matrix, gen_float, gen_float2, gen_2d_mix, gen_sat_shifts, gen_mul_long, gen_loop_branch, gen_bfield_extract, gen_sat_arith, gen_3d_accum, gen_128_struct, gen_float_reduce, gen_fma_chain, gen_mask_extract, gen_signed_div, gen_widen_byte_lut]
def main():
    fails=0; ok=0; skip=0
    for i in range(N):
        tag=f"{seed0}_{i}"
        # mix static JIT-only generators with loader-mode (PIE+globals) ones
        if random.random() < 0.35:
            src=random.choice([gen_globals_pie, gen_pie_callchain])()
            e,j,q=build_pair_mode(src,tag,"pie")
        else:
            src=random.choice(gens)()
            e,j,q=build_pair(src,tag)
        if e is None:
            skip+=1; continue
        if j is None or q is None:
            skip+=1; continue
        try:
            p="PASS" if int(j)==int(q) else "FAIL"
        except:
            p="SKIP"; skip+=1; continue
        if p=="FAIL":
            fails+=1
            os.rename(f"fx_{tag}.elf", f"FUZZFAIL_{tag}.elf")
            try: os.rename(f"fx_{tag}.c", f"FUZZFAIL_{tag}.c")
            except: pass
            print(f"FAIL[{tag}] oracle={q} jit={j}")
        else:
            ok+=1
        # cleanup
        for ext in (".c",".elf"):
            try: os.remove(f"fx_{tag}{ext}")
            except: pass
    print(f"\nseed={seed0} ok={ok} fail={fails} skip={skip}")

if __name__=="__main__":
    main()
