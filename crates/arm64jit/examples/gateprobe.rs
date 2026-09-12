fn main() {
    for w in [0x4e710e10u32, 0x4e220c20, 0x0e220c20, 0x2e220c20, 0x0e620c20, 0x0ea20c20] {
        println!("{w:08x} -> {:?}", arm64jit::decode::decode(w));
    }
}