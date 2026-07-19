/* libandroidicu.so — Android ICU external stubs.
 *
 * These are stub implementations of the Android ICU4C external API.
 * Real Android ICU is an APEX module; we provide no-op stubs that
 * let libxml2, libsqlite, libmedia, etc. load without crashing. */

#include <stddef.h>
#include <stdint.h>

/* ===== ucnv (ICU converter subsystem) ===== */
void *ucnv_open_android(const char *name, void *err) {
    (void)name; if (err) *(int*)err = 0; return NULL;
}
void ucnv_close_android(void *conv) { (void)conv; }
void ucnv_setToUCallBack_android(void *c, void *a, void *b, void **x, void **y, void *e) {
    (void)c;(void)a;(void)b;(void)x;(void)y; if(e)*(int*)e=0;
}
void ucnv_setFromUCallBack_android(void *c, void *a, void *b, void **x, void **y, void *e) {
    (void)c;(void)a;(void)b;(void)x;(void)y; if(e)*(int*)e=0;
}
void UCNV_TO_U_CALLBACK_STOP_android(void *ctx, void *args) { (void)ctx;(void)args; }
void UCNV_FROM_U_CALLBACK_STOP_android(void *ctx, void *args) { (void)ctx;(void)args; }
void ucnv_convertEx_android(void *tc, void *sc,
    void **t, const void *tl, void **s, const void *sl,
    void *pb, void **ps, void **pt, void *pl,
    int r, int f, void *e) {
    (void)tc;(void)sc;(void)t;(void)tl;(void)s;(void)sl;
    (void)pb;(void)ps;(void)pt;(void)pl;(void)r;(void)f; if(e)*(int*)e=0;
}
int ucnv_getNextUChar_android(void *conv, const void **src, const void *srclim, void *err) {
    (void)conv;(void)src;(void)srclim; if(err)*(int*)err=0; return -1;
}

/* ===== ucsdet (ICU charset detection) ===== */
void *ucsdet_open_android(void *err) { if(err)*(int*)err=0; return (void*)1; }
void ucsdet_setText_android(void *det, const void *text, int len, void *err) {
    (void)det;(void)text;(void)len; if(err)*(int*)err=0;
}
void ucsdet_close_android(void *det) { (void)det; }
void *ucsdet_detect_android(void *det, void *err) {
    (void)det; if(err)*(int*)err=0; return NULL;
}
void *ucsdet_detectAll_android(void *det, int *count, void *err) {
    (void)det; if(err)*(int*)err=0; if(count)*count=0; return NULL;
}
const char *ucsdet_getName_android(void *match, void *err) {
    (void)match; if(err)*(int*)err=0; return "UTF-8";
}
int ucsdet_getConfidence_android(void *match, void *err) {
    (void)match; if(err)*(int*)err=0; return 0;
}

/* ===== ucol (ICU collation) ===== */
void *ucol_open_android(const void *loc, void *err) {
    (void)loc; if(err)*(int*)err=0; return (void*)2;
}
void ucol_close_android(void *col) { (void)col; }
void ucol_setStrength_android(void *col, int strength) { (void)col;(void)strength; }
void ucol_setAttribute_android(void *col, int attr, int value, void *err) {
    (void)col;(void)attr;(void)value; if(err)*(int*)err=0;
}
int ucol_strcoll_android(void *col, const void *s1, int len1, const void *s2, int len2) {
    (void)col;(void)s1;(void)len1;(void)s2;(void)len2; return 0;
}
int ucol_strcollIter_android(void *col, const void *s1, const void *s2, void *err) {
    (void)col;(void)s1;(void)s2; if(err)*(int*)err=0; return 0;
}

/* ===== ubrk (ICU break iterator) ===== */
void *ubrk_open_android(int type, const void *loc, const void *text, int len, void *err) {
    (void)type;(void)loc;(void)text;(void)len; if(err)*(int*)err=0; return (void*)3;
}
void ubrk_close_android(void *bi) { (void)bi; }
int ubrk_first_android(void *bi) { (void)bi; return -1; }
int ubrk_current_android(void *bi) { (void)bi; return -1; }
int ubrk_next_android(void *bi) { (void)bi; return -1; }

/* ===== uregex (ICU regular expressions) ===== */
void *uregex_open_android(const void *pat, int patlen, unsigned flags, void *err, void *status) {
    (void)pat;(void)patlen;(void)flags;(void)status; if(err)*(int*)err=0; return (void*)4;
}
void uregex_close_android(void *re) { (void)re; }
void uregex_setText_android(void *re, const void *text, int len, void *err) {
    (void)re;(void)text;(void)len; if(err)*(int*)err=0;
}
int uregex_matches_android(void *re, int start, void *err) {
    (void)re;(void)start; if(err)*(int*)err=0; return 0;
}

/* ===== ustring (ICU string utilities) ===== */
int u_foldCase_android(void *s1, int len1, const void *s2, int len2, unsigned opts) {
    (void)s1;(void)len1;(void)s2;(void)len2;(void)opts; return len1;
}
int u_isspace_android(int c) { (void)c; return 0; }
int u_strToUTF8_android(void *dest, int destcap, int *destlen,
    const void *src, int srclen, void *err) {
    (void)dest;(void)destcap;(void)src;(void)srclen;
    if(destlen)*destlen=0; if(err)*(int*)err=0; return 0;
}
void *u_strToUpper_android(void *dst, int dstcap, int *dstlen,
    const void *src, int srclen, const void *loc, void *err) {
    (void)dst;(void)dstcap;(void)src;(void)srclen;(void)loc;
    if(dstlen)*dstlen=0; if(err)*(int*)err=0; return dst;
}
void *u_strToLower_android(void *dst, int dstcap, int *dstlen,
    const void *src, int srclen, const void *loc, void *err) {
    (void)dst;(void)dstcap;(void)src;(void)srclen;(void)loc;
    if(dstlen)*dstlen=0; if(err)*(int*)err=0; return dst;
}
const char *u_errorName_android(int code) { (void)code; return "U_ZERO_ERROR"; }
void uiter_setUTF8_android(void *iter, const char *s, int len) {
    (void)iter;(void)s;(void)len;
}