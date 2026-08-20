// SPDX-License-Identifier: MIT
//
// elf_disco.h - public API for the version-agnostic ELF discovery module.
#ifndef ELF_DISCO_H
#define ELF_DISCO_H

#include <stddef.h>
#include <stdint.h>
#include <elf.h>

typedef struct RoboELF {
    uint8_t        *buf;
    size_t          size;
    Elf64_Phdr     *ph;
    int             nph;
    Elf64_Sym      *dynsym;       /* DT_SYMTAB (file image pointer) */
    char           *dynstr;       /* DT_STRTAB (file image pointer) */
    size_t          dynstr_size;
    Elf64_Rela     *rela;         /* DT_RELA   (file image pointer) */
    size_t          rela_num;
    Elf64_Rela     *pltrel;       /* DT_JMPREL (file image pointer) */
    size_t          pltrel_num;
} RoboELF;

#ifdef __cplusplus
extern "C" {
#endif

/* Parse the ELF at `path`. Returns 0 on success. */
int       robo_open(RoboELF *e, const char *path);
void      robo_close(RoboELF *e);

/* Runtime address (base + r_offset) of the GOT/reloc slot for `name`, or 0. */
uintptr_t robo_got(const RoboELF *e, uint64_t base, const char *name);

/* Runtime [start,end) writable range (PT_GNU_RELRO, else last PF_W LOAD). */
int       robo_relro_range(const RoboELF *e, uint64_t base,
                           uint64_t *out_start, uint64_t *out_end);

/* Exact ADRP target page. */
uint64_t  arm64_adrp_target(uint32_t insn, uint64_t pc);

/* Runtime target of the first out-of-line BL inside function `fn`. */
uint64_t  robo_first_bl(const RoboELF *e, uint64_t fn, uint32_t max);

#ifdef __cplusplus
}
#endif

#endif /* ELF_DISCO_H */