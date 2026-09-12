fn main() {
    for (w,l) in [(0x4e422420u32,"fcmeq8h"),(0x6ec22420,"fcmgt8h"),(0x6e422420,"fcmge8h"),(0x2ec22462,"real fcmgt4h v2"),(0x6eca26ac,"real fcmgt8h v12"),(0x0ef8f820,"fabs4h"),(0x6e1424a3,"mov v3.s[2] COLLISION-TEST")] {
        println!("{l}: {:08x} {:?}", w, arm64jit::decode::decode(w));
    }
}