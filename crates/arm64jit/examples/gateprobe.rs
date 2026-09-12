fn main() {
    for (w,l) in [(0x6eca26acu32,"real fcmgt8h v12"),(0x6ecb258b,"real fcmgt8h"),(0x2ecb258b,"real fcmgt4h"),(0x4e422420,"fcmeq8h"),(0x6ec22420,"fcmgt8h"),(0x6e1424a3,"mov .S[idx]")] {
        println!("{l}: {:?}", arm64jit::decode::decode(w));
    }
}
