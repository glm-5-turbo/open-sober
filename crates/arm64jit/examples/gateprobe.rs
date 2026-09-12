fn main() {
    for (w,l) in [(0x4ef8e843u32,"fcmlt8h real"),(0x0ef8d820,"fcmeq4h"),(0x0ef8c820,"fcmgt4h"),(0x2ef8c820,"fcmge4h"),(0x0ef8e820,"fcmlt4h"),(0x2ef8d820,"fcmle4h"),(0x0ef8f820,"fabs4h must-not")] {
        println!("{l}: {:?}", arm64jit::decode::decode(w));
    }
}