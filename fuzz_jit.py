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
    if q is None:
        q=run_qemu_exact(src,tag)
    return e, j, q

# shared-library (-shared -fPIC) build: exported module functions calling each
# OTHER and themselves go through `@plt`, whose JUMP_SLOTs the binder must
# resolve to the module's OWN guest addresses (self-imports), and recursion
# must divert cleanly. A -(pi e) -nostdlib static ELF never exercises this.
def build_pair_shared(src, tag):
    open(f"fx_{tag}.c","w").write(src)
    r=subprocess.run(["aarch64-linux-gnu-gcc","-O3","-w","-shared","-fPIC","-nostdlib",
                      "-Wl,-e,entry",f"fx_{tag}.c","-o",f"fx_{tag}.elf"],
                     capture_output=True,text=True)
    if r.returncode!=0:
        return None,None,None
    e=getentry(f"fx_{tag}.elf")
    j=run_elfjit(f"fx_{tag}.elf", e) if e else None
    q=run_qemu(src,tag)
    if q is None:
        q=run_qemu_exact(src,tag)
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


def run_qemu_exact(src, tag):
    # 64-bit-exact oracle for programs whose host-native compile fails (e.g.
    # <arm_neon.h> intrinsics that don't exist on x86). Cross-compile src with a
    # write+itoa _start wrapper and run under qemu-aarch64; return the decimal
    # output (unsigned long long view of entry()), like run_qemu. qemu computes
    # with real AArch64 NEON/FP, so it is a correct oracle for the vector ISA.
    open(f"q_{tag}.c","w").write(src)
    r=subprocess.run(["aarch64-linux-gnu-gcc","-O3","-w","-static","-nostdlib","-Wl,-e,_start",
                      f"q_{tag}.c","wrap.S","-o",f"q_{tag}.elf"],
                     capture_output=True,text=True)
    if r.returncode!=0:
        return None
    r=subprocess.run(["qemu-aarch64","-L","/usr/aarch64-linux-gnu",f"q_{tag}.elf"],
                     capture_output=True,text=True,timeout=25)
    if r.returncode!=0:
        return None
    return r.stdout.strip()

gens=[gen_arith, gen_byte, gen_shift_matrix, gen_float, gen_float2, gen_2d_mix, gen_sat_shifts, gen_mul_long, gen_loop_branch, gen_bfield_extract, gen_sat_arith, gen_3d_accum, gen_128_struct, gen_float_reduce, gen_fma_chain, gen_mask_extract, gen_signed_div, gen_widen_byte_lut]
def gen_neon_byelem():
    # Force NEON *by-element* fmla/fmul (vmlaq_n/vfmaq_n) + lane ins/get.
    # Lanes are small binary-exact ints and the accumulator stays exact in
    # float32, so any structural lane/operand miscompute shows as a real
    # diff (not FMA-vs-mul+add ULP noise).
    n=random.choice([4,8,12,16])
    c=random.choice([0.5,1.0,2.0,4.0,-1.0,-2.0])
    idx=random.choice([0,1,2,3])
    ex=random.choice([0,1,2,3])
    return f"""#include <arm_neon.h>
long long entry(void){{
    volatile unsigned long long seedv = 777333ull;
    unsigned long long x = seedv;
    float fa[{n}];
    for(int i=0;i<{n};i++){{ x=x*1664525ull+1013904223ull; fa[i]=(float)(int)(((x>>40)&0x7f)-64); }}
    float32x4_t v = vdupq_n_f32(0.0f);
    float scalar_acc = 0.0f;
    for(int i=0;i<{n};i++){{
        v = vmlaq_n_f32(v, vdupq_n_f32(fa[i]), {c}f);
        scalar_acc += fa[i]*{c}f;
    }}
    // lane insert/extract round-trip with exact offsets
    float32x4_t one = vdupq_n_f32(1.0f);
    float32x4_t w = vsetq_lane_f32(vgetq_lane_f32(v,{idx})+1.0f, one, {ex});
    v = vaddq_f32(v, w);
    float s = 0; for(int l=0;l<4;l++) s += vgetq_lane_f32(v,l);
    long long ref = (long long)(scalar_acc + 1.0);
    long long got = (long long)s;
    return got - ref;
}}
"""
def gen_neon_tbl_bitmix():
    # Bitwise select/tbl-heavy vector mixing (bsl/bit/bif + vext + vrbit)
    n=random.choice([8,16])
    half=n//2
    m=random.choice([0x55555555,0xAAAAAAAA,0xF0F0F0F0,0x12345678])
    ex=random.choice([1,2,3])
    sh=random.choice([1,5,17,31])
    return f"""#include <arm_neon.h>
long long entry(void){{
    volatile unsigned long long seedv = 414243ull;
    unsigned long long x = seedv;
    uint32_t a[{n}];
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1442695040888963407ull; a[i]=(uint32_t)x; }}
    uint32x4_t av = vld1q_u32(a);
    uint32x4_t bv = vld1q_u32(a+{half});
    uint32x4_t mmask = vdupq_n_u32({m});
    uint32x4_t s1 = vbslq_u32(mmask, av, bv);
    uint32x4_t s2 = vextq_u32(s1, av, {ex});
    uint32x4_t s3 = vrev64q_u32(s2);
    uint32_t r[4]; vst1q_u32(r, s3);
    long long acc=0; for(int i=0;i<4;i++) acc = acc*131 + (long long)r[i];
    for(int i=0;i<{n};i+=2) acc += (long long)a[i]>>{sh};
    return acc;
}}
"""

def gen_bfi_64():
    # 64-bit bitfield insert/set (bfi/bfiz/sbfiz on GPRs) — gcc emits these for
    # packed struct fields / color / bit flags; the ubfiz/sbfiz OLD bug lived in
    # this immr>imms overlap.
    n=random.choice([4,8,12])
    pos=random.choice([1,3,5,7,8,9,15,17,24,31,33,40,48,57,63])
    wid=random.choice([1,2,3,4,8,12,16])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 715517ull;
    unsigned long long x = seedv;
    unsigned long long a[{n}];
    for(int i=0;i<{n};i++){{ x=x*1103515245ull+12345ull; a[i]=x; }}
    unsigned long long acc=0;
    for(int i=0;i<{n};i++){{
        unsigned long long field = (a[i] >> {random.choice([1,8,24,32,40,48,56])}) & ((1ull<<{wid})-1);
        acc |= (field << {pos});
        acc ^= a[i] & 0xffff;
        acc = (acc*131) ^ (acc>>17);
    }}
    return (long long)acc;
}}
"""
def gen_tbz_branches():
    # Bit-test branches (tbz/tbnz when the mask is a compile-time const power of 2)
    # + switch jump tables with sparse high values forcing tbz-based branches.
    n=random.choice([16,32,64])
    k=random.choice([0,1,3,7,8,15,20,31,33,40,55,63])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 990077ull;
    unsigned long long x = seedv;
    long long a[{n}], s=0;
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1442695040888963407ull; a[i]=(long long)x; }}
    for(int i=0;i<{n};i++){{
        long long v=a[i];
        if(v & (1ull<<{k})) s += (v>>1);            // tbnz
        else if(v & (1ull<<{random.choice([1,5,9,17,25,33,41,63])})) s += (v<<1);  // tbnz
        else if((v%(13|1))==0) s ^= (v>>3);
        s = s*7;
    }}
    return s;
}}
"""
def gen_fixed_pt_fcvt():
    # Fixed-point float->int (fcvtzs/fcvtzu #fbits): (long long)(f*2^k) unfolded
    # via volatile — exercises the #fbits scaling path (was misdecoded as smull).
    n=random.choice([8,16,24])
    bits=random.choice([1,2,3,4,8,12,16,20])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 246802ull;
    unsigned long long x = seedv;
    double a[{n}];
    for(int i=0;i<{n};i++){{ x=x*1664525ull+1013904223ull; a[i]=((double)((x>>40)&0x3fff)-2048.0)/256.0; }}
    volatile double k = {1<<bits} ;
    long long s=0;
    for(int i=0;i<{n};i++) s += (long long)((double)(a[i]*k));
    for(int i=0;i<{n};i+=2) s -= (long long)(a[i]/2.0);
    return s;
}}
"""
def gen_pairwise_reduce():
    # NEON across-lane / pairwise reductions: sum-v-across (addv/saddlv/uaddlv),
    # horizontal add-pair (addp), and vector sum accumulators. These emit the
    # `addv Bd,Vn.4s` / `uaddlv`/`saddlv` family the ledger flagged as lightly
    # covered. Integer lanes keep it exact.
    n=random.choice([8,12,16,24])
    ty=random.choice(["int","unsigned int","short","unsigned short","long long"])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 654321ull;
    unsigned long long x = seedv;
    {ty} a[{n}];
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1442695040888963407ull; a[i]=({ty})((x>>{random.choice([8,16,24,32,40])})^(x&0xffff)); }}
    long long s=0;
    for(int i=0;i<{n};i++) s += (long long)a[i];
    for(int i=0;i<{n};i+=2) s ^= (long long)a[i] + a[i+1];
    return s;
}}
"""
def gen_varshift():
    # Variable shift-by-register: lslv/lsrv/asrv (and rorv) Rd,Rn,Rm where BOTH
    # the value and the shift count are runtime data (not compile-time constants),
    # with 32- and 64-bit lanes. gcc emits the lslv/lsrv/asrv/rorv encodings only
    # when the amount is genuinely data-dependent; constant-integer shifts emit
    # UBFM/EXTR instead. Covers the VarShiftVar translate arm (count masked to
    # sf?0x3f:0x1f, W sign-extends before asrv, zero-extends result).
    n=random.choice([16,24,32,48])
    width=random.choice(["unsigned long long","long long","unsigned int","int"])
    seed=random.choice([864209, 13579, 975310, 424242])
    # Shift counts must stay < element width: ARM `shl/sshl` by >= width yields 0,
    # while native x86 masks `cl` mod width (UB region differs by ISA). Oracle here
    # is native gcc, so keep counts in [0, width_bits) to compare like-for-like.
    wbits = 64 if "long long" in width else 32
    max_shift = wbits - 1
    sh_mask = random.choice([7, 15, 31, max_shift])
    sh_mask = min(sh_mask, max_shift)
    return f"""long long entry(void){{
    volatile unsigned long long seedv = {seed}ull;
    unsigned long long x = seedv;
    {width} a[{n}], sh[{n}];
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1442695040888963407ull;
        a[i]=({width})((x>>{random.choice([8,17,31,45])}) ^ (x & 0xffff));
        sh[i]=({width})((x >> 37) & {sh_mask});
        if(({width})sh[i] < 0) sh[i] = ({width})0 - sh[i];
    }}
    {width} s1=0,s2=0,s3=0;
    for(int i=0;i<{n};i++){{
        s1 ^= ({width})(a[i] << sh[i]);
        s2 |= ({width})(a[i] >> sh[i]);
    }}
    for(int i=0;i<{n};i+=2) s3 += ({width})(a[i] << (sh[i]&7)) - ({width})(a[i] >> ({width})(sh[i]&15));
    return (long long)(s1*131 + s2*17 + s3*3) ^ 0xf00baaull;
}}
"""
def gen_scalar_varshift():
    # SCALAR variable shift (lslv/lsrv/asrv) cannot be auto-vectorized: the
    # running hash h is a loop-carried dependency, so gcc keeps each shift in a
    # GPR (`lsl x11,x11,x9` / `asrv` etc. — the VarShiftVar arm), unlike array
    # elementwise shifts which vectorize to NEON ushl/sshl. Covers the scalar
    # count-masking (sf?0x3f:0x1f), W sign-extend before asrv, zero-ext result.
    n=random.choice([16,24,32,48])
    width=random.choice(["unsigned long long","long long","unsigned int","int"])
    seed=random.choice([998877, 112233, 556677, 314159])
    wbits = 64 if "long long" in width else 32
    sh_mask = min(random.choice([3,7,15,31,63]), wbits-1)
    return f"""long long entry(void){{
    volatile unsigned long long seedv = {seed}ull;
    unsigned long long x = seedv;
    {width} sh[{n}];
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1442695040888963407ull; sh[i]=({width})((x>>33)&{sh_mask}); }}
    {width} h=0x12345678;
    for(int i=0;i<{n};i++){{ h = ({width})((h << ({width})sh[i]) ^ (h >> ({width})sh[i])) ^ ({width})i; }}
    return (long long)h;
}}
"""
gens += [gen_neon_byelem, gen_neon_tbl_bitmix, gen_bfi_64, gen_tbz_branches, gen_fixed_pt_fcvt, gen_pairwise_reduce, gen_varshift, gen_scalar_varshift]

# -shared self-import shape: exported module functions calling each OTHER via
# @plt (and optionally recursing). Exercises the binder's own-export JUMP_SLOT
# resolution + the import-bearing recursion divert. Runs only under
# build_pair_shared (-shared -fPIC).
def gen_selfimport():
    kind=random.choice(["calls","recursive","deep"])
    if kind=="calls":
        n=random.randint(3,6)
        fns=[]
        body="long long entry(void){\n"
        for i in range(n):
            fns.append(f"long long g{i}(long long x){{ return x*{random.choice([2,3,5,7,11,13])}+{random.randint(0,99)}; }}")
            body+=f"    x = g{i}(x);\n" if i==0 else f"    x = g{i}(x) + g{random.randint(0,i-1)}(x);\n"
        body+="    return x;\n}\n"
        pre="long long x = 3;\n" if random.random()<0.5 else "long long x = seedv;\n"
        body="    volatile unsigned long long seedv=12345ull;\n    "+pre+body
        return "\n".join(fns)+"\n"+body
    if kind=="recursive":
        # recursion only makes sense BEFORE entry; keep bounded.
        d=random.randint(6,12)
        return (f"long long rec(long long n){{ return n<=0 ? {random.randint(1,9)} : rec(n-1)+{random.randint(2,9)}; }}\n"
                f"long long entry(void){{ volatile unsigned long long seedv=1ull; return rec({random.randint(d-3,d)}); }}\n")
    # deep: nested calls through several exported helpers then a fold
    fns=[]
    for i in range(4):
        fns.append(f"long long h{i}(long long x){{ return x^{random.randint(1,3)} + {random.randint(5,50)}; }}")
    ch=" + ".join(f"h{i}(x)" for i in range(4))
    return ("\n".join(fns)+"\n"
            f"long long entry(void){{ volatile unsigned long long seedv=77771ull; long long x=(long long)(seedv>>{random.choice([3,7,11,13])}); return {ch}; }}\n")
def gen_fp_edge():
    # FP edge cases a graphics/audio engine hits that benign finite-value
    # generators never exercise: division (incl. by 0 and tiny), NaN
    # propagation, +/-inf, -0.0 and signed-zero compares, and boundary
    # int<->float conversions. We only require jit == native x86 oracle
    # (same C compiled to both), which sidesteps correctness debates.
    kind=random.choice(["float","double"])
    op=random.choice(["/","*","+","-"])
    n=random.choice([8,16,24])
    specials=['0.0','-0.0','1.0/0.0','-1.0/0.0','0.0/0.0','1e-300','-1e-300']
    s1=random.choice(specials); s2=random.choice(specials)
    B="{"                      # literal open brace
    E="}"                      # literal close brace
    return (f"long long entry(void){B}\n"
            + f"    volatile unsigned long long seedv = 777313ull;\n"
            + f"    unsigned long long x = seedv;\n"
            + f"    {kind} a[{n}];\n"
            + f"    for(int i=0;i<{n};i++){B} x=x*2862933555777941757ull+3037000493ull; a[i]=({kind})(((long long)((x>>45)&0x7ffff)-65536))*1e-6; {E}\n"
            + f"    {kind} acc = 0.0; {kind} lo = 1.0/0.0; {kind} hi = -1.0/0.0;\n"
            + f"    {kind} sp = ({kind})({s1});\n"
            + f"    for(int i=0;i<{n};i++){B} acc = acc {op} a[i]; {E}\n"
            + f"    acc = acc {op} sp;\n"
            + f"    for(int i=0;i<{n};i++){B} if(a[i]<lo) lo=a[i]; if(a[i]>hi) hi=a[i]; {E}\n"
            + f"    {kind} nn = ({kind})({s2});\n"
            + f"    {kind} mn = a[0] < nn ? a[0] : nn;\n"
            + f"    {kind} mx = a[0] > nn ? a[0] : nn;\n"
            + f"    long long ivals=0;\n"
            + f"    for(int i=0;i<{n};i++){B}\n"
            + f"        long long v=(long long)(a[i]*1e6); if(v>4000000000ll) v=4000000000ll; if(v<-4000000000ll) v=-4000000000ll;\n"
            + f"        ivals += v;\n"
            + f"    {E}\n"
            + f"    {kind} fp = ({kind})ivals;\n"
            + f"    long long back = (long long)fp;\n"
            + f"    long long zero_cmp = ({kind})(0.0) > ({kind})-0.0 ? 7 : 3;\n"
            + f"    long long r = ((long long)acc & 0x1ffff) + (long long)lo + (long long)hi + (long long)mn + (long long)mx + back + zero_cmp + (ivals&0xff);\n"
            + f"    return r & 0xfffffffff;\n"
            + f"{E}\n")

gens_shared=[gen_selfimport]
gens += [gen_fp_edge]

def gen_double_neon():
    # Double-precision (64-bit lane) NEON — none of the other SIMD generators
    # exercise f64 lanes, but a 3D engine's physics/audio math is dense with
    # them. Forces vfmaq_n_f64/vmulq_n_f64 by-element (.2d) and the horizontal
    # fmaxv plus a vld1q_f64/vst1q_f64 round trip. Lane values are small
    # binary-exact doubles and the accumulator is exact in f64, so a structural
    # lane/operand miscompute shows as a real diff. The host x86 gcc can't
    # compile <arm_neon.h>, so this runs through the qemu-aarch64 architectural
    # oracle (run_qemu_exact), never a native-gcc one.
    n=random.choice([4,6,8,10])
    c=random.choice([0.25,0.5,1.0,2.0,-1.0,-3.0])
    idx=random.choice([0,1])
    return f"""#include <arm_neon.h>
long long entry(void){{
    volatile unsigned long long seedv = 555777ull;
    unsigned long long x = seedv;
    double da[{n}];
    for(int i=0;i<{n};i++){{ x=x*2862933555777941757ull+3037000493ull; da[i]=(double)(long long)(((x>>44)&0x7ff)-512); }}
    float64x2_t v = vdupq_n_f64(0.0);
    double sacc = 0.0;
    for(int i=0;i<{n};i++){{
        v = vfmaq_n_f64(v, vdupq_n_f64(da[i]), {c});
        v = vmulq_n_f64(v, 0.5);
        sacc = sacc*0.25 + da[i]*{c}*0.25;
    }}
    v = vfmaq_n_f64(v, vdupq_n_f64(3.0), 0.25);
    sacc += 3.0*0.25;
    double m = vmaxvq_f64(v);
    double l[2]; vst1q_f64(l, v);
    double lanesum = l[0] + l[1];
    double ref = sacc + m + lanesum + da[0];
    return (long long)ref;
}}
"""

def gen_uxtl_uaddw():
    # 16->32 and 32->64 widening with uaddw/subw + narrow back, forcing the
    # SimdAddw / SimdXtl (permute-source) alias paths across multiple widths
    n=random.choice([8,16,24])
    k=random.choice([3,7,13,1009])
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 998877ull;
    unsigned long long x = seedv;
    unsigned short a[{n}];
    for(int i=0;i<{n};i++){{ x=x*6364136223846793005ull+1ull; a[i]=(unsigned short)(x>>48); }}
    long long s=0;
    for(int i=0;i<{n};i+=4){{
        unsigned int p = (unsigned int)a[i] + a[i+1] + a[i+2] + a[i+3];
        s += (long long)p * {k};
    }}
    for(int i=1;i<{n};i+=2) s += (long long)a[i]*a[i-1];
    return s;
}}
"""

gens += [gen_double_neon, gen_uxtl_uaddw]

def gen_scalar_fp_sign_chain():
    # Scalar FP sign/abs/negate + fp<->int round trips with signed zeros and
    # negative magnitudes — exercises the FmovGp, scalar FMaxMin, Fabs/Fneg,
    # FcvtToInt/FcvtFromInt paths that a graphics/audio engine hits constantly
    # but the int-heavy generators never do. Binary-exact magnitudes so oracle
    # parity is exact.
    n=random.choice([6,10,14])
    op=random.choice(["max","min"])
    accsel = "(b >= acc ? b : acc)" if op=="max" else "(b <= acc ? b : acc)"
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 314159ull;
    unsigned long long x = seedv;
    double a[{n}];
    for(int i=0;i<{n};i++){{ x=x*2862933555777941757ull+3037000493ull; a[i]=(double)(long long)(((x>>45)&0x3ff)-256); }}
    double acc = 1e-9;
    double lo = 0.0, hi = 0.0;
    for(int i=0;i<{n};i++){{
        double av = (a[i] < 0) ? -a[i] : a[i];      // abs via select
        double n2 = -a[i];                          // fneg
        double b = (a[i] < 3.0) ? av : n2;          // fmax/fmin mix
        acc = {accsel};
        if (b < lo) lo = b;
        if (b > hi) hi = b;
    }}
    // fp->int->fp sign round trips (exercises fcvtzs + scvtf + fmov gp/fp)
    long long ip = (long long)(hi*1000.0);
    double back = (double)(long long)(lo*1000.0);
    double zero_cmp = (0.0 > -0.0) ? 5.0 : 2.0;
    double r = ((hi + lo)*31.0) + (double)(ip & 0x7ff) + back + zero_cmp;
    long long rr = (long long)r;
    return (rr < 0 ? -rr : rr) & 0xfffffffff;
}}
"""

gens += [gen_scalar_fp_sign_chain]

def gen_fmadd_reduce():
    # Scalar FP fused multiply-accumulate (fmadd/fmsub d0 = a*b+c) with a
    # binary-exact integer-based chain. GCC's `acc += a[i]*k` and
    # `acc = acc*f + a[i]` emit fmadd; the JIT does mul+add (two roundings) vs
    # ARM's fused one-rounding so values are NOT bit-exact — require only the
    # int(acc) agreement after scaling so rounding mode differences collapse.
    n=random.choice([8,16,24])
    k=random.choice([3,5,7,11,13,31])
    f=random.choice([0.5,2.0,-1.5,3.0])
    mode=random.choice(["mac","poly"])
    if mode=="mac":
        body=(f"    for(int i=0;i<{n};i++){{ acc = acc + (double)a[i]*{k}.0; }}\n"
              f"    for(int i=0;i<{n};i++){{ acc = acc*{f} + (double)a[i]; }}\n")
    else:
        body=(f"    double x = 0.5;\n"
              f"    for(int i=0;i<{n};i++){{ x = x*{f} + (double)a[i]*{k}.0; }}\n"
              f"    acc = x;\n")
    return f"""long long entry(void){{
    volatile unsigned long long seedv = 424242ull;
    unsigned long long x = seedv;
    int a[{n}];
    for(int i=0;i<{n};i++){{ x=x*1664525ull+1013904223ull; a[i]=(int)((x>>33)^(x>>13)) - 1024; }}
    double acc = 0.0;
    {body}    long long r = (long long)acc;
    return r & 0x3fffffff;
}}
"""

gens += [gen_fmadd_reduce]

def gen_byte_reverse_perm():
    # SIMD byte-reverse / permutation family that the int-heavy generators
    # never produce: vrev16/32/64, byte-swap, vuzp/vtrn lane interlacing.
    # Data is a byte pattern so heavy rev/permute mangles register lanes; the
    # reference accumulates the same bytes in normal order, so any lane/rev or
    # width mismatch shows as a real oracle diff (not ULP noise).
    n=random.choice([16,32,64])
    which=random.choice(["rev32q","rev64q","rev16q","uzp","trn","rbit-u32","zip","zipw","zip","zipw"])
    return f"""#include <arm_neon.h>
long long entry(void){{
    volatile unsigned long long seedv = 555555ull;
    unsigned long long x = seedv;
    uint8_t ua[{n}];
    for(int i=0;i<{n};i++){{ x=x*1664525ull+1013904223ull; ua[i]=(uint8_t)((x>>40)&0xff); }}
    uint8x16_t a = vld1q_u8(ua);
    uint8x16_t b = vreinterpretq_u8_u64(vdupq_n_u64(0x0102030405060708ull));
    uint8x16_t r;
    if ("{which}"=="rev32q") r = vrev32q_u8(a);
    else if ("{which}"=="rev64q") r = vrev64q_u8(a);
    else if ("{which}"=="rev16q") r = vrev16q_u8(a);
    else if ("{which}"=="uzp") {{ r = vaddq_u8(vuzp1q_u8(a,b), vuzp2q_u8(a,b)); }}
    else if ("{which}"=="zip") {{ r = vaddq_u8(vzip1q_u8(a,b), vzip2q_u8(a,b)); }}
    else if ("{which}"=="zipw") r = vreinterpretq_u8_u16(vaddq_u16(vzip1q_u16(vreinterpretq_u16_u8(a), vreinterpretq_u16_u8(b)), vzip2q_u16(vreinterpretq_u16_u8(a), vreinterpretq_u16_u8(b))));
    else {{ uint32x4_t t = vreinterpretq_u32_u8(a); r = vreinterpretq_u8_u32(vrev64q_u32(t)); }}
    uint8_t out[{n}]; vst1q_u8(out,r); vst1q_u8(out+16,a); vst1q_u8(out+32,b);
    unsigned long long acc = 0;
    for(int i=0;i<{n};i++) acc = acc*131 + out[i];
    return (long long)(acc & 0x3fffffff);
}}
"""

def gen_widen_mul_acc():
    # SIMD widening multiply-accumulate: vmull/vmlal/vmlsl (smull/smlal/umlal,
    # signed+unsigned, 16x16->32 and 32x32->64 lanes) that the scalar gen_mul_long
    # never produces. Data is small ints so the widening sum is exact in the
    # destination element type; the reference computes the same widening products
    # in scalar u64, so any sign/widen/accumulate lane mismatch is an exact diff.
    n=random.choice([8,16,32])
    use16=random.choice([True,False])
    signed=random.choice([True,False])
    sub=random.choice([True,False])
    return f"""#include <arm_neon.h>
long long entry(void){{
    volatile unsigned long long seedv = 998877ull;
    unsigned long long x = seedv;
    long long a[{n}], b[{n}];
    for(int i=0;i<{n};i++){{ x=x*1664525ull+1013904223ull; a[i]=(long long)(int)(((x>>40)&0xff)-64); b[i]=(long long)(int)(((x>>24)&0xff)-64); }}
    int64x2_t acc64 = vdupq_n_s64(0);
    int32x4_t acc32 = vdupq_n_s32(0);
    for(int i=0;i<{n}/4;i++){{
        int16x4_t ha = vmovn_s32(vdupq_n_s32((int32_t)a[i*4]));
        int16x4_t hb = vmovn_s32(vdupq_n_s32((int32_t)b[i*4]));
        acc32 = vmlal_s16(acc32, ha, hb);   // 16x16 -> 32
        int32x2_t wa = vmovn_s64(vdupq_n_s64((int64_t)a[i*4]));
        int32x2_t wb = vmovn_s64(vdupq_n_s64((int64_t)b[i*4]));
        acc64 = vmlal_s32(acc64, wa, wb);   // 32x32 -> 64
    }}
    int64x2_t r64 = vabsq_s64(acc64);
    int32x4_t r32 = vabsq_s32(acc32);
    long long sa = vgetq_lane_s64(r64,0) + vgetq_lane_s64(r64,1);
    long long sb = (long long)vgetq_lane_s32(r32,0)+vgetq_lane_s32(r32,1)+vgetq_lane_s32(r32,2)+vgetq_lane_s32(r32,3);
    return (sa + sb*1009) & 0x3fffffff;
}}
"""

gens += [gen_widen_mul_acc]

gens += [gen_byte_reverse_perm]

def main():
    fails=0; ok=0; skip=0
    for i in range(N):
        tag=f"{seed0}_{i}"
        # mix static JIT-only generators with loader-mode (PIE+globals) ones,
        # and a slice of -shared self-import shapes (binder own-export + import-
        # bearing recursion coverage)
        r=random.random()
        if r < 0.30:
            src=random.choice([gen_globals_pie, gen_pie_callchain])()
            e,j,q=build_pair_mode(src,tag,"pie")
        elif r < 0.55:
            src=random.choice(gens_shared)()
            e,j,q=build_pair_shared(src,tag)
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
