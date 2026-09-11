#!/usr/bin/env python3
"""Generate crates/{egl,glesv2}-wrapper forwarders from exact Mesa header signatures."""
import json, re, pathlib

ROOT = pathlib.Path("/home/hermes-worker/runs/open-sober")

GL_SCALAR = {
    "GLenum": "u32", "GLboolean": "u8", "GLbitfield": "u32", "GLbyte": "i8",
    "GLubyte": "u8", "GLshort": "i16", "GLushort": "u16", "GLint": "i32",
    "GLuint": "u32", "GLsizei": "i32", "GLfloat": "f32", "GLclampf": "f32",
    "GLdouble": "f64", "GLchar": "c_char", "GLintptr": "isize",
    "GLsizeiptr": "isize", "GLint64": "i64", "GLuint64": "u64",
}

EGL_SCALAR = {
    "EGLBoolean": "u32", "EGLint": "i32", "EGLenum": "u32",
    "EGLAttrib": "isize", "EGLTime": "u64", "EGLNativeDisplayType": "isize",
    "EGLNativeWindowType": "isize", "EGLNativePixmapType": "isize",
    "EGLClientBuffer": "*mut c_void", "EGLDisplay": "*mut c_void",
    "EGLConfig": "*mut c_void", "EGLSurface": "*mut c_void",
    "EGLContext": "*mut c_void", "EGLImage": "*mut c_void",
    "EGLSync": "*mut c_void", "__eglMustCastToProperFunctionPointerType": "*mut c_void",
}

# C identifiers that are Rust keywords -> suffix underscore.
KEYWORDS = {"type", "fn", "in", "match", "loop", "move", "ref", "self", "mut", "where", "box", "async", "await", "dyn"}

RESERVED = {"type", "ref"}

def make_ptr_type(base_rust, n_star, outer_const):
    """n_star==0 -> scalar; else build a raw pointer type.
    outer_const only applies to the outermost star for `const T *`."""
    if n_star == 0:
        return base_rust
    mut = "const" if outer_const else "mut"
    ty = f"*{mut} {base_rust}"
    # inner levels are always mutable pointee pointers (rare: only ** glue)
    for _ in range(1, n_star):
        ty = f"*mut {ty}"
    return ty


def parse_type(tok, scalar_map):
    tok = tok.strip()
    # Strip every `const` qualifier (leading `const T *` and inner `T *const *`).
    is_const = "const" in tok
    t = re.sub(r"\bconst\b", " ", tok)
    t = re.sub(r"\s+", " ", t).strip()
    n_star = t.count("*")
    base = t.replace("*", "").strip()
    if base == "void":
        base_rust = "c_void"
    elif base in scalar_map:
        base_rust = scalar_map[base]
    else:
        base_rust = "u32"
    return make_ptr_type(base_rust, n_star, is_const)


HDR = """// AUTO-GENERATED from Mesa headers (see gen_forward.py / scangl.py). Do not edit by hand.
// Each entry resolves the real Mesa symbol once and forwards the call.
#![allow(non_snake_case, unused_imports, clippy::missing_safety_doc)]
use std::ffi::{c_char, c_void};
"""


def emit(path, libname, funcs, scalar_map):
    lines = [HDR]
    for name in sorted(funcs.keys()):
        ret, args = funcs[name]
        rret = "()" if ret.strip() == "void" else parse_type(ret, scalar_map)
        arg_names, arg_tys = [], []
        for a in args.split(","):
            if not a.strip():
                continue
            a = a.strip()
            m = re.match(r"^(.*?)([A-Za-z_]\w*)$", a)
            ty_part, nm = m.group(1).strip(), m.group(2)
            if nm in RESERVED:
                nm = nm + "_"
            arg_names.append(nm)
            arg_tys.append(parse_type(ty_part, scalar_map))
        sig = ", ".join(f"{n}: {t}" for n, t in zip(arg_names, arg_tys))
        tylist = ", ".join(arg_tys)
        fty = f"unsafe extern \"C\" fn({tylist}) -> {rret}" if rret != "()" else f"unsafe extern \"C\" fn({tylist})"
        call = ", ".join(arg_names)
        ret_open = f" -> {rret}" if rret != "()" else ""
        lines.append(f"#[unsafe(no_mangle)]\npub unsafe extern \"C\" fn {name}({sig}){ret_open} {{")
        lines.append(f"    let real: {fty} = crate::dl::sym::<{fty}>(\"{libname}\", \"{name}\");")
        lines.append(f"    unsafe {{ real({call}) }}\n}}\n")
    pathlib.Path(path).write_text("\n".join(lines) + "\n")
    print(f"wrote {path}: {len(funcs)} functions, {len(lines)} lines")


gles = json.load(open("/tmp/gles_sigs.json"))
gles.pop("glCompressedTexImage2D", None)
gles.pop("glCompressedTexSubImage2D", None)
emit(ROOT / "crates/glesv2-wrapper/src/generated_gles.rs", "libGLESv2.so.2", gles, GL_SCALAR)

egl = json.load(open("/tmp/egl_sigs.json"))
emit(ROOT / "crates/egl-wrapper/src/generated_egl.rs", "libEGL.so.1", egl, EGL_SCALAR)

print("intercepted (hand-written): glCompressedTexImage2D, glCompressedTexSubImage2D")