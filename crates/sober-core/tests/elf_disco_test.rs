// SPDX-License-Identifier: MIT
//
// Integration test for the version-agnostic ELF discovery module
// (crates/sober-core/src/elf_disco.c).
//
// It cross-compiles a small ARM64 .so that imports pthread_cond_wait /
// pthread_cond_timedwait / pthread_mutex_lock, then runs the host
// elf_disco harness and asserts robo_got() returns exactly the GOT
// relocation slots reported by `readelf -rW`. This proves the discovery
// mechanism that replaces hardcoded offsets works against a real ARM64 ELF,
// without needing a Roblox APK.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn run(prog: &str, args: &[&str]) -> std::process::Output {
    Command::new(prog)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {}", prog, e))
}

fn stdout(o: &std::process::Output) -> String {
    assert!(
        o.status.success(),
        "cmd failed:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// Parse `readelf -rW` -> map symbol -> GOT vaddr (r_offset).
fn readelf_got(readelf: &str) -> HashMap<String, u64> {
    let mut m = HashMap::new();
    for line in readelf.lines() {
        let t: Vec<&str> = line.split_whitespace().collect();
        if t.len() >= 5 && t[2] == "R_AARCH64_JUMP_SLOT" {
            let off = u64::from_str_radix(t[0].trim_start_matches("0x"), 16).unwrap();
            // t[3] is the symbol value; t[4] is the symbol name.
            let sym = t[4].split('@').next().unwrap().to_string();
            m.insert(sym, off);
        }
    }
    m
}

/// Parse harness output `robo_got(name)=0x…` -> map.
fn parse_harness(s: &str) -> HashMap<String, u64> {
    let mut m = HashMap::new();
    for line in s.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("robo_got(") {
            if let Some(eq) = rest.find(")=") {
                let sym = &rest[..eq];
                let val = &rest[eq + 2..];
                if let Some(hx) = val.strip_prefix("0x") {
                    m.insert(sym.to_string(), u64::from_str_radix(hx, 16).unwrap());
                }
            }
        }
    }
    m
}

#[test]
fn elf_disco_resolves_import_got_slots() {
    let crate_dir = crate_root();
    let src = crate_dir.join("src");
    let out = crate_dir.join("target").join("elf_disco_it");
    std::fs::create_dir_all(&out).unwrap();

    // 1. ARM64 fixture lib.
    let module = "
        #include <pthread.h>
        int f(void) {
            pthread_cond_t c = PTHREAD_COND_INITIALIZER;
            pthread_mutex_t m = PTHREAD_MUTEX_INITIALIZER;
            pthread_mutex_lock(&m);
            pthread_cond_wait(&c, &m);
            pthread_cond_timedwait(&c, &m, 0);
            pthread_mutex_unlock(&m);
            pthread_cond_destroy(&c);
            pthread_mutex_destroy(&m);
            return 0;
        }
    ";
    let lib_c = out.join("fixture.c");
    std::fs::write(&lib_c, module).unwrap();
    let lib_so = out.join("libfixture.so");
    let o = run("aarch64-linux-gnu-gcc", &[
        "-shared", "-fPIC", "-o", lib_so.to_str().unwrap(), lib_c.to_str().unwrap(),
    ]);
    stdout(&o);
    assert!(lib_so.exists(), "fixture .so not created");

    // 2. Ground truth via readelf.
    let rel = stdout(&run(
        "aarch64-linux-gnu-readelf",
        &["-rW", lib_so.to_str().unwrap()],
    ));
    let truth = readelf_got(&rel);
    assert!(truth.contains_key("pthread_cond_wait"), "fixture missing import");

    // 3. Compile elf_disco.c for the host.
    let disco_o = out.join("elf_disco.o");
    let o = run("cc", &[
        "-c", "-O1", "-I", src.to_str().unwrap(),
        "-o", disco_o.to_str().unwrap(),
        src.join("elf_disco.c").to_str().unwrap(),
    ]);
    assert!(o.status.success(), "host compile elf_disco failed");

    // 4. Harness main.
    let harness_c = out.join("harness.c");
    std::fs::write(
        &harness_c,
        r#"
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include "elf_disco.h"
int main(int argc, char**argv) {
    if (argc < 2) return 2;
    RoboELF e;
    if (robo_open(&e, argv[1]) != 0) { fprintf(stderr, "open fail\n"); return 2; }
    uint64_t B = 0x10000;
    const char* syms[] = {"pthread_cond_wait","pthread_cond_timedwait",
                          "pthread_mutex_lock","pthread_mutex_unlock",
                          "pthread_cond_destroy","pthread_mutex_destroy"};
    for (int i = 0; i < 6; i++) {
        uintptr_t v = robo_got(&e, B, syms[i]);
        printf("robo_got(%s)=0x%lx\n", syms[i], (unsigned long)v);
    }
    robo_close(&e);
    return 0;
}
"#,
    )
    .unwrap();
    let harness_bin = out.join("harness");
    let o = run("cc", &[
        "-I", src.to_str().unwrap(),
        "-o", harness_bin.to_str().unwrap(),
        harness_c.to_str().unwrap(),
        disco_o.to_str().unwrap(),
    ]);
    assert!(o.status.success(), "harness link failed");

    // 5. Run harness.
    let hout = stdout(&run(harness_bin.to_str().unwrap(), &[lib_so.to_str().unwrap()]));
    let got = parse_harness(&hout);

    // 6. Compare: for each known import, discovered (=B + r_offset) slot.
    for (sym, got_slot) in &got {
        if let Some(truth_off) = truth.get(sym) {
            assert_eq!(*got_slot, 0x10000 + truth_off, "slot mismatch for {}", sym);
        } else if *got_slot != 0 {
            panic!("robo_got found unexpected symbol {}", sym);
        }
    }
}