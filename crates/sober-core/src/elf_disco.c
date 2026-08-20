// SPDX-License-Identifier: MIT
//
// elf_disco.c - version-agnostic ELF discovery for the Open Sober JNI shim.
//
// The runtime used to hardcode virtual offsets into ONE Roblox libroblox.so
// build (GOT slots for pthread_cond_wait / pthread_mutex_lock, the stack
// canary slot, the RELRO range, BSS guard/freq fields, JNI_OnLoad patch
// sites). Those offsets are wrong for any other build, which is exactly why
// the runtime broke on this machine (no matching APK available).
//
// This module re-derives offsets from a given ELF at runtime so the shim
// adapts to whatever libroblox.so it is handed:
//
//   robo_open / robo_close    read+parse an ARM64 ELF .so from disk
//   robo_got                  exact runtime address of an import's GOT slot
//   robo_relro_range          writable PT_LOAD / PT_GNU_RELRO run
//   arm64_adrp_target          exact ADRP page decode (unit tested)
//   robo_addr_of_bl            runtime target of a BL in a function body
//
// Parsing reads the on-disk file only; the caller passes the runtime `base`
// so . and results are independent of the load address.

#define _GNU_SOURCE
#include "elf_disco.h"
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#include <stdlib.h>

/* Translate a guest vaddr (relative to base) to a file offset, or -1. */
static uintptr_t robo_file_off(const RoboELF *e, uint64_t vaddr) {
    for (int i = 0; i < e->nph; ++i) {
        const Elf64_Phdr *p = &e->ph[i];
        if (p->p_type != PT_LOAD) continue;
        uint64_t seg = p->p_vaddr;
        uint64_t end = p->p_vaddr + (p->p_filesz ? p->p_filesz : p->p_memsz);
        if (vaddr >= seg && vaddr < end)
            return p->p_offset + (vaddr - seg);
    }
    return (uint64_t)-1;
}

/* ------------------------------------------------------------------------- */
/* Parse                                                                */
/* ------------------------------------------------------------------------- */

/* Parse the ELF at `path` into `e`. Returns 0 on success. On failure the
 * caller should not call robo_close() with a reason to clean up the mmap. */
int robo_open(RoboELF *e, const char *path) {
    memset(e, 0, sizeof(*e));

    int fd = open(path, O_RDONLY);
    if (fd < 0)
        return -1;
    struct stat st;
    if (fstat(fd, &st) != 0 || st.st_size < (off_t)sizeof(Elf64_Ehdr)) {
        close(fd);
        return -1;
    }
    e->size = (size_t)st.st_size;
    e->buf = mmap(NULL, e->size, PROT_READ, MAP_PRIVATE, fd, 0);
    close(fd);
    if (e->buf == MAP_FAILED) {
        e->buf = NULL;
        return -1;
    }

    Elf64_Ehdr *eh = (Elf64_Ehdr *)e->buf;
    if (memcmp(eh->e_ident, ELFMAG, SELFMAG) != 0 ||
        eh->e_ident[EI_CLASS] != ELFCLASS64 ||
        eh->e_ident[EI_DATA] != ELFDATA2LSB ||
        eh->e_type != ET_DYN) {
        munmap(e->buf, e->size);
        e->buf = NULL;
        return -1;
    }

    e->ph  = (Elf64_Phdr *)(e->buf + eh->e_phoff);
    e->nph = eh->e_phnum;

    /* find PT_DYNAMIC */
    const Elf64_Phdr *dyn_ph = NULL;
    for (int i = 0; i < e->nph; ++i)
        if (e->ph[i].p_type == PT_DYNAMIC) { dyn_ph = &e->ph[i]; break; }
    if (!dyn_ph) {
        munmap(e->buf, e->size);
        e->buf = NULL;
        return -1;
    }

    uint64_t doff = robo_file_off(e, dyn_ph->p_vaddr);
    if (doff == (uint64_t)-1) {
        munmap(e->buf, e->size);
        e->buf = NULL;
        return -1;
    }
    const Elf64_Dyn *dyn = (const Elf64_Dyn *)(e->buf + doff);
    size_t dyncount = (size_t)(dyn_ph->p_filesz / sizeof(Elf64_Dyn));

    Elf64_Addr symtab_va = 0, strtab_va = 0, rela_va = 0, jmprel_va = 0;
    Elf64_Xword strsz = 0, relasz = 0, jmsz = 0;
    Elf64_Sxword plt_type = DT_RELA;

    for (size_t i = 0; i < dyncount; ++i) {
        Elf64_Sxword tag = dyn[i].d_tag;
        Elf64_Xword  val = dyn[i].d_un.d_val;
        switch ((int)tag) {
        case DT_SYMTAB:   symtab_va = val; break;
        case DT_STRTAB:   strtab_va = val; break;
        case DT_STRSZ:    strsz = val; break;
        case DT_RELA:     rela_va = val; break;
        case DT_RELASZ:   relasz = val; break;
        case DT_JMPREL:   jmprel_va = val; break;
        case DT_PLTRELSZ: jmsz = val; break;
        case DT_PLTREL:   plt_type = val; break;
        default: break;
        }
    }

    if (symtab_va) {
        uint64_t o = robo_file_off(e, symtab_va);
        e->dynsym = o == (uint64_t)-1 ? NULL : (Elf64_Sym *)(e->buf + o);
    }
    if (strtab_va) {
        uint64_t o = robo_file_off(e, strtab_va);
        e->dynstr = o == (uint64_t)-1 ? NULL : (char *)(e->buf + o);
        e->dynstr_size = (size_t)strsz;
    }
    if (rela_va) {
        uint64_t o = robo_file_off(e, rela_va);
        e->rela     = o == (uint64_t)-1 ? NULL : (Elf64_Rela *)(e->buf + o);
        e->rela_num = (size_t)(relasz / sizeof(Elf64_Rela));
    }
    if (jmprel_va && plt_type == DT_RELA) {
        uint64_t o = robo_file_off(e, jmprel_va);
        e->pltrel     = o == (uint64_t)-1 ? NULL : (Elf64_Rela *)(e->buf + o);
        e->pltrel_num = (size_t)(jmsz / sizeof(Elf64_Rela));
    }

    return 0;
}

void robo_close(RoboELF *e) {
    if (e && e->buf) {
        munmap(e->buf, e->size);
        e->buf = NULL;
    }
}

/* ------------------------------------------------------------------------- */
/* GOT resolution                                                            */
/* ------------------------------------------------------------------------- */

/* Return the runtime address (base + r_offset) of the GOT/reloc slot that
 * symbol `name` resolves to, or 0 if not found. Works for imports in both
 * DT_RELA and DT_JMPREL. */
uintptr_t robo_got(const RoboELF *e, uint64_t base, const char *name) {
    for (int table = 0; table < 2; ++table) {
        const Elf64_Rela *rl = table ? e->pltrel : e->rela;
        size_t           n   = table ? e->pltrel_num : e->rela_num;
        for (size_t i = 0; i < n; ++i) {
            Elf64_Word symidx = ELF64_R_SYM(rl[i].r_info);
            Elf64_Word type   = ELF64_R_TYPE(rl[i].r_info);
            if (!e->dynsym || !e->dynstr) continue;
            if (symidx == 0) continue; /* R_*_NONE */
            const Elf64_Sym *s = &e->dynsym[symidx];
            if (s->st_name >= e->dynstr_size) continue;
            if (strcmp(e->dynstr + s->st_name, name) != 0) continue;
            if (type == R_AARCH64_JUMP_SLOT || type == R_AARCH64_GLOB_DAT)
                return base + rl[i].r_offset;
        }
    }
    return 0;
}

/* ------------------------------------------------------------------------- */
/* RELRO range                                                               */
/* ------------------------------------------------------------------------- */

/* Return the runtime [*out_start, *out_end) writable range (PT_GNU_RELRO,
 * else the last writable PT_LOAD). Returns 0 on success, -1 otherwise. */
int robo_relro_range(const RoboELF *e, uint64_t base,
                     uint64_t *out_start, uint64_t *out_end) {
    for (int i = 0; i < e->nph; ++i)
        if (e->ph[i].p_type == PT_GNU_RELRO) {
            *out_start = base + e->ph[i].p_vaddr;
            *out_end   = base + e->ph[i].p_vaddr + e->ph[i].p_memsz;
            return 0;
        }
    int best = -1;
    for (int i = 0; i < e->nph; ++i)
        if (e->ph[i].p_type == PT_LOAD && (e->ph[i].p_flags & PF_W))
            best = i;
    if (best < 0) return -1;
    *out_start = base + e->ph[best].p_vaddr;
    *out_end   = base + e->ph[best].p_vaddr + e->ph[best].p_memsz;
    return 0;
}

/* ------------------------------------------------------------------------- */
/* Minimal AArch64 decode helpers (shared; unit tested).                     */
/* ------------------------------------------------------------------------- */

/* ADRP: returns the target page (4KiB aligned). `pc` is the address of the
 * adrp instruction. */
uint64_t arm64_adrp_target(uint32_t insn, uint64_t pc) {
    uint64_t immhi = (insn >> 5)  & 0x7ffffULL;   /* immhi[18:0] */
    uint64_t immlo = (insn >> 29) & 0x3ULL;       /* immlo[1:0]  */
    int64_t  off   = (int64_t)((immhi << 2) | immlo);
    off = (off << 12) >> 12;                       /* sign-extend 21 bits */
    return (pc & ~0xfffULL) + (uint64_t)off;
}

/* Find the BL target inside a runtime function body.
 * `e` the opened ELF, `fn` the runtime address of a function, `max` the
 * number of instructions to scan before giving up. Returns the runtime
 * target address of the first arLS BL/B to a code address at least one page
 * away from the function start (i.e. an out-of-line call), or 0.
 */
uint64_t robo_first_bl(const RoboELF *e, uint64_t fn, uint32_t max) {
    /* BL: 100101 (26-bit imm). BR/BLR/BRA/BLRAA are 110101 1 ... */
    uint64_t off = robo_file_off(e, fn);
    if (off == (uint64_t)-1) return 0;
    uint64_t limit = off + (uint64_t)max * 4;
    if (limit >= e->size) limit = e->size;
    for (uint64_t p = off; p + 4 <= limit; p += 4) {
        uint32_t insn;
        memcpy(&insn, e->buf + p, 4);
        if ((insn & 0xfc000000u) == 0x94000000u) {  /* BL */
            /* signed imm26 */
            int64_t imm26 = (int64_t)(insn & 0x03ffffffu);
            if (imm26 & 0x02000000u) imm26 |= ~((int64_t)0x03ffffffu);
            uint64_t pc = fn + (p - off);            /* runtime pc of the bl */
            uint64_t target = pc + (uint64_t)(imm26 << 2);
            /* out-of-line call heuristic: at least 1 page away */
            if ((target ^ pc) & ~0xfffULL)
                return target;
        }
    }
    return 0;
}