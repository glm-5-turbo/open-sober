/* libicu.so — Android ICU stubs (LIBICU_31 version).
 *
 * Provides stub implementations of ICU4C functions that Android
 * libraries reference. Real ICU is system-provided; these stubs
 * prevent crashes during dlopen(). */

#include <stddef.h>
#include <stdint.h>

/* ===== ubidi (Unicode Bidirectional Algorithm) ===== */
void *ubidi_open(void) { return (void*)0x100; }
void ubidi_close(void *bi) { (void)bi; }
void ubidi_setPara(void *bi, const void *text, int len, int level, void *embed, void *info) {
    (void)bi;(void)text;(void)len;(void)level;(void)embed;(void)info;
}
void ubidi_setClassCallback(void *bi, void *cb, void *ctx, void **oldcb, void **oldctx, void *err) {
    (void)bi;(void)cb;(void)ctx;(void)oldcb;(void)oldctx; if(err)*(int*)err=0;
}
int ubidi_countRuns(void *bi, void *err) {
    (void)bi; if(err)*(int*)err=0; return 0;
}
int ubidi_getVisualRun(void *bi, int run, int *start, int *len) {
    (void)bi;(void)run; if(start)*start=0; if(len)*len=0; return 0;
}
int ubidi_getParaLevel(void *bi) { (void)bi; return 0; }

/* ===== unorm2 (Unicode Normalization) ===== */
void *unorm2_getNFDInstance(void *err) { if(err)*(int*)err=0; return (void*)0x200; }
int unorm2_getRawDecomposition(void *norm2, int c, void *buf, int cap, void *err) {
    (void)norm2;(void)c;(void)buf;(void)cap; if(err)*(int*)err=0; return 0;
}

/* ===== uchar (Unicode Character Properties) ===== */
int u_hasBinaryProperty(int c, int which) { (void)c;(void)which; return 0; }
int u_charDirection(int c) { (void)c; return 0; }
int u_charType(int c) { (void)c; return 13; }  /* U_UNASSIGNED */
int u_getIntPropertyValue(int c, int which) { (void)c;(void)which; return 0; }
int u_getIntPropertyMaxValue(int which) { (void)which; return 0; }
int u_charMirror(int c) { (void)c; return 0; }
int u_iscntrl(int c) { (void)c; return 0; }
int uscript_getScript(int c, void *err) { (void)c; if(err)*(int*)err=0; return 0; }

/* ===== uloc (ICU Locale) ===== */
int uloc_canonicalize(const void *loc, void *buf, int cap, void *err) {
    (void)loc;(void)buf;(void)cap; if(err)*(int*)err=0; return 0;
}
int uloc_addLikelySubtags(const void *loc, void *buf, int cap, void *err) {
    (void)loc;(void)buf;(void)cap; if(err)*(int*)err=0; return 0;
}
int uloc_toLanguageTag(const void *loc, void *buf, int cap, int strict, void *err) {
    (void)loc;(void)buf;(void)cap;(void)strict; if(err)*(int*)err=0; return 0;
}
int uloc_forLanguageTag(const void *tag, void *buf, int cap, int *parsed, void *err) {
    (void)tag;(void)buf;(void)cap;(void)parsed; if(err)*(int*)err=0; return 0;
}

/* ===== ubrk (Break Iterator) ===== */
void *ubrk_open(int type, const void *loc, const void *text, int len, void *err) {
    (void)type;(void)loc;(void)text;(void)len; if(err)*(int*)err=0; return (void*)0x300;
}
void ubrk_close(void *bi) { (void)bi; }
int ubrk_next(void *bi) { (void)bi; return -1; }
int ubrk_isBoundary(void *bi, int offset) { (void)bi;(void)offset; return 1; }
int ubrk_following(void *bi, int offset) { (void)bi;(void)offset; return -1; }
void ubrk_setUText(void *bi, void *text, void *err) {
    (void)bi;(void)text; if(err)*(int*)err=0;
}

/* ===== utext (Unicode Text Iterator) ===== */
void *utext_openUChars(void *ut, const void *s, int len, void *err) {
    (void)s;(void)len; (void)ut; if(err)*(int*)err=0; return (void*)0x400;
}
void utext_close(void *ut) { (void)ut; }

/* ===== ustring (Unicode String Utilities) ===== */
const char *u_errorName(int code) { (void)code; return "U_ZERO_ERROR"; }

/* ===== Additional uchar stubs needed by libpdfium.so ===== */
int u_isalnum(int c) { (void)c; return 0; }
int u_isalpha(int c) { (void)c; return 0; }
int u_isspace(int c) { (void)c; return 0; }
int u_toupper(int c) { (void)c; return 0; }
int u_tolower(int c) { (void)c; return 0; }