fn main() {
    for (w,l) in [(0x4e709cf0u32,"mul8h real"),(0x4e629c20,"mul8h"),(0x0e629c20,"mul4h"),(0x4ea29c20,"mul4s"),(0x4ee29c20,"mul2d?")] {
        println!("{l}: {:08x} {:?}", w, arm64jit::decode::decode(w));
    }
}
