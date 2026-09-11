long long entry(void){
    volatile unsigned long long seedv = 777313ull;
    unsigned long long x = seedv;
    double a[8];
    for(int i=0;i<8;i++){ x=x*2862933555777941757ull+3037000493ull; a[i]=(double)(((long long)((x>>45)&0x7ffff)-65536))*1e-6; }
    double acc = 0.0; double lo = 1.0/0.0; double hi = -1.0/0.0;
    double sp = (double)(-1.0/0.0);
    for(int i=0;i<8;i++){ acc = acc + a[i]; }
    acc = acc + sp;
    for(int i=0;i<8;i++){ if(a[i]<lo) lo=a[i]; if(a[i]>hi) hi=a[i]; }
    double nn = (double)(0.0/0.0);
    double mn = a[0] < nn ? a[0] : nn;
    double mx = a[0] > nn ? a[0] : nn;
    long long ivals=0;
    for(int i=0;i<8;i++){
        long long v=(long long)(a[i]*1e6); if(v>4000000000ll) v=4000000000ll; if(v<-4000000000ll) v=-4000000000ll;
        ivals += v;
    }
    double fp = (double)ivals;
    long long back = (long long)fp;
    long long zero_cmp = (double)(0.0) > (double)-0.0 ? 7 : 3;
    long long r = ((long long)acc & 0x1ffff) + (long long)lo + (long long)hi + (long long)mn + (long long)mx + back + zero_cmp + (ivals&0xff);
    return r & 0xfffffffff;
}
