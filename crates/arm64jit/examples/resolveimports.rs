//! Diagnostic: report how many of `libroblox.so`'s PLT JUMP_SLOT imports
//! `bind_image_plt` ties to host thunks (libc/libm/float/bionic/graphics-stub).
//! This is the same code the boot path (`elfjit`) runs; this example surfaces
//! it in a readable form against the real binary.

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/code-agent/.cache/open-sober/libs/libroblox.so".to_string());
    let el = unsafe { libloader::elf::load_elf_image(std::path::Path::new(&path)) }
        .expect("load_elf_image");
    let (bound, unbound) = arm64jit::plt::bind_image_plt(&el);
    println!("== {path} PLT JUMP_SLOT import summary ==");
    println!("resolved-to-host: {bound}  |  unbound: {unbound}  (total {})", bound + unbound);
}