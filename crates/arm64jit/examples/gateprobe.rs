fn main() {
    for (w,l) in [(0x1ee14000u32,"fneg.h0"),(0x1ee54000,"frintm.h0"),(0x4ef9d801,"frecpe.8h"),(0x6ef9d801,"frsqrte.8h"),(0x0ef9d801,"frecpe.4h"),(0x2e219800,"frintx.2s"),(0x6e619821,"frintx.2d"),(0x6e799800,"frintx.8h")] {
        println!("{l}: {:?}", arm64jit::decode::decode(w));
    }
}
