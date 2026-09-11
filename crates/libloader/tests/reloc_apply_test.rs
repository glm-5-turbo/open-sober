// SPDX-License-Identifier: MIT
//
// Integration test: `load_elf_image` must materialize `R_AARCH64_RELATIVE`
// data relocations for a PIE/shared object before the code can dereference
// pointer globals (vtables, `.data.rel.ro` pointers). The real Roblox APK
// libs and their Android/GSI dependencies are ET_DYN with such relocations
// (often APS2-packed via DT_ANDROID_RELA); this test proves the Rust loader
// applies the standard-DT_RELA case against a real cross-compiled aarch64
// `.so`, so the JIT path can boot those libraries without the QEMU path's
// external `unpack_rela.py`.
//
// Skips cleanly when `aarch64-linux-gnu-gcc` / `aarch64-linux-gnu-readelf`
// are absent (kept green on hosts without the cross toolchain).

use std::path::PathBuf;
use std::process::Command;

fn cross_gcc() -> bool {
    Command::new("aarch64-linux-gnu-gcc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn workdir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("open-sober-reloc-regress-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Parse `readelf -r --use-dynamic` and return every `(r_offset, addend)`
/// `R_AARCH64_RELATIVE` relocation. Example line:
///   `00000001fe28  000000000403 R_AARCH64_RELATIV  600`
/// (readelf truncates the type to 19 columns, so match the prefix; the addend
/// is the last token, which may carry a `+`/`-` prefix.)
fn all_relatives(readelf: &str) -> Vec<(u64, i64)> {
    readelf
        .lines()
        .filter_map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            if t.len() < 4 || !t[2].starts_with("R_AARCH64_REL") {
                return None;
            }
            let off = u64::from_str_radix(t[0].trim_start_matches("0x"), 16).ok()?;
            let addend_tok = t[t.len() - 1];
            let neg = addend_tok.starts_with('-');
            let digits = addend_tok.trim_start_matches(['+', '-']);
            let mut addend = i128::from_str_radix(digits, 16).ok()?;
            if neg {
                addend = -addend;
            }
            Some((off, addend as i64))
        })
        .collect()
}

#[test]
fn load_elf_image_applies_relative_relocations() {
    if !cross_gcc() {
        eprintln!("skipping: aarch64-linux-gnu-gcc not available");
        return;
    }
    let wd = workdir();

    // A shared object with a pointer global initialized to the address of a
    // private data global: the compiler must emit a R_AARCH64_RELATIVE reloc
    // for the `ptr` slot (stored in .data.rel.ro), with addend = &secret's
    // link-time vaddr. Under ET_DYN, that slot must become `load_bias + A`.
    let src_c = wd.join("rel.c");
    std::fs::write(
        &src_c,
        "static int secret = 0x11223344;\n\
         int *ptr = &secret;\n\
         int __attribute__((noinline)) deref(void){ return *ptr; }\n\
         int __attribute__((noinline)) init2(void){ return deref(); }\n",
    )
    .unwrap();
    let so = wd.join("librel.so");
    let o = Command::new("aarch64-linux-gnu-gcc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(&so)
        .arg(&src_c)
        .output()
        .unwrap_or_else(|e| panic!("cross-gcc failed: {e}"));
    assert!(o.status.success(), "cross-gcc failed:\n{}", String::from_utf8_lossy(&o.stderr));
    assert!(so.exists(), "fixture .so not created");

    // Ground truth from readelf.
    let rel = Command::new("aarch64-linux-gnu-readelf")
        .args(["-r", "--use-dynamic"])
        .arg(&so)
        .output()
        .expect("readelf missing");
    let rel_txt = String::from_utf8_lossy(&rel.stdout).into_owned();
    let relatives = all_relatives(&rel_txt);
    assert!(
        !relatives.is_empty(),
        "no R_AARCH64_RELATIVE in:\n{}",
        rel_txt
    );

    // Load through the JIT loader path.
    let el = unsafe { libloader::elf::load_elf_image(&so) }.expect("load_elf_image");
    assert!(el.info.is_pie, "fixture should be ET_DYN/PIE");

    // The RELATIVE value must be the runtime load bias + addend, realized at
    // the guest address of each r_offset (guest==host for load_elf_image).
    let load_bias = el.base_addr as u64 - el.info.base_load_addr;
    for (r_offset, addend) in &relatives {
        let addend = *addend;
        let target = el.guest_of(*r_offset);
        let expected = load_bias.wrapping_add(addend as u64);
        let got = unsafe { *(target as *const u64) };
        assert_eq!(
            got, expected,
            "slot {target:#x} (r_offset {r_offset:#x}) should be load_bias({load_bias:#x}) + addend({addend:#x}) = {expected:#x}, got {got:#x} — RELATIVE not applied"
        );
    }
    assert!(relatives.len() >= 2, "expected several RELATIVE relocs, got {}", relatives.len());

    let _ = std::fs::remove_dir_all(&wd);
}