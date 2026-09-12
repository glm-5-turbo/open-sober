fn main() {
    for (w,l) in [(0x6e35dc44u32,"fmul4s"),(0x6e34dc40,"fmul4s b"),(0x4ea2ce24,"fmls4s"),(0x6ea0fa19,"fneg4s"),(0x4ebfce25,"fmls4s c"),(0x6e36dcc6,"fmul4s d")] {
        println!("{l}: {:?}", arm64jit::decode::decode(w));
    }
}