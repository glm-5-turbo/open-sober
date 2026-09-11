// AUTO-GENERATED from Mesa headers (see gen_forward.py / scangl.py). Do not edit by hand.
// Each entry resolves the real Mesa symbol once and forwards the call.
#![allow(non_snake_case, unused_imports, clippy::missing_safety_doc)]
use std::ffi::{c_char, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glActiveTexture(texture: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glActiveTexture");
    unsafe { real(texture) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glAttachShader(program: u32, shader: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glAttachShader");
    unsafe { real(program, shader) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBindAttribLocation(program: u32, index: u32, name: *const c_char) {
    let real: unsafe extern "C" fn(u32, u32, *const c_char) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *const c_char)>("libGLESv2.so.2", "glBindAttribLocation");
    unsafe { real(program, index, name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBindBuffer(target: u32, buffer: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glBindBuffer");
    unsafe { real(target, buffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBindFramebuffer(target: u32, framebuffer: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glBindFramebuffer");
    unsafe { real(target, framebuffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBindRenderbuffer(target: u32, renderbuffer: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glBindRenderbuffer");
    unsafe { real(target, renderbuffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBindTexture(target: u32, texture: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glBindTexture");
    unsafe { real(target, texture) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBlendColor(red: f32, green: f32, blue: f32, alpha: f32) {
    let real: unsafe extern "C" fn(f32, f32, f32, f32) = crate::dl::sym::<unsafe extern "C" fn(f32, f32, f32, f32)>("libGLESv2.so.2", "glBlendColor");
    unsafe { real(red, green, blue, alpha) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBlendEquation(mode: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glBlendEquation");
    unsafe { real(mode) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBlendEquationSeparate(modeRGB: u32, modeAlpha: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glBlendEquationSeparate");
    unsafe { real(modeRGB, modeAlpha) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBlendFunc(sfactor: u32, dfactor: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glBlendFunc");
    unsafe { real(sfactor, dfactor) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBlendFuncSeparate(sfactorRGB: u32, dfactorRGB: u32, sfactorAlpha: u32, dfactorAlpha: u32) {
    let real: unsafe extern "C" fn(u32, u32, u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, u32, u32)>("libGLESv2.so.2", "glBlendFuncSeparate");
    unsafe { real(sfactorRGB, dfactorRGB, sfactorAlpha, dfactorAlpha) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBufferData(target: u32, size: isize, data: *const c_void, usage: u32) {
    let real: unsafe extern "C" fn(u32, isize, *const c_void, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, isize, *const c_void, u32)>("libGLESv2.so.2", "glBufferData");
    unsafe { real(target, size, data, usage) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glBufferSubData(target: u32, offset: isize, size: isize, data: *const c_void) {
    let real: unsafe extern "C" fn(u32, isize, isize, *const c_void) = crate::dl::sym::<unsafe extern "C" fn(u32, isize, isize, *const c_void)>("libGLESv2.so.2", "glBufferSubData");
    unsafe { real(target, offset, size, data) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCheckFramebufferStatus(target: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libGLESv2.so.2", "glCheckFramebufferStatus");
    unsafe { real(target) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glClear(mask: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glClear");
    unsafe { real(mask) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glClearColor(red: f32, green: f32, blue: f32, alpha: f32) {
    let real: unsafe extern "C" fn(f32, f32, f32, f32) = crate::dl::sym::<unsafe extern "C" fn(f32, f32, f32, f32)>("libGLESv2.so.2", "glClearColor");
    unsafe { real(red, green, blue, alpha) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glClearDepthf(d: f32) {
    let real: unsafe extern "C" fn(f32) = crate::dl::sym::<unsafe extern "C" fn(f32)>("libGLESv2.so.2", "glClearDepthf");
    unsafe { real(d) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glClearStencil(s: i32) {
    let real: unsafe extern "C" fn(i32) = crate::dl::sym::<unsafe extern "C" fn(i32)>("libGLESv2.so.2", "glClearStencil");
    unsafe { real(s) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glColorMask(red: u8, green: u8, blue: u8, alpha: u8) {
    let real: unsafe extern "C" fn(u8, u8, u8, u8) = crate::dl::sym::<unsafe extern "C" fn(u8, u8, u8, u8)>("libGLESv2.so.2", "glColorMask");
    unsafe { real(red, green, blue, alpha) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCompileShader(shader: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glCompileShader");
    unsafe { real(shader) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCopyTexImage2D(target: u32, level: i32, internalformat: u32, x: i32, y: i32, width: i32, height: i32, border: i32) {
    let real: unsafe extern "C" fn(u32, i32, u32, i32, i32, i32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, u32, i32, i32, i32, i32, i32)>("libGLESv2.so.2", "glCopyTexImage2D");
    unsafe { real(target, level, internalformat, x, y, width, height, border) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCopyTexSubImage2D(target: u32, level: i32, xoffset: i32, yoffset: i32, x: i32, y: i32, width: i32, height: i32) {
    let real: unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, i32, i32)>("libGLESv2.so.2", "glCopyTexSubImage2D");
    unsafe { real(target, level, xoffset, yoffset, x, y, width, height) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCreateProgram(void: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libGLESv2.so.2", "glCreateProgram");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCreateShader(type_: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libGLESv2.so.2", "glCreateShader");
    unsafe { real(type_) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCullFace(mode: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glCullFace");
    unsafe { real(mode) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDeleteBuffers(n: i32, buffers: *const u32) {
    let real: unsafe extern "C" fn(i32, *const u32) = crate::dl::sym::<unsafe extern "C" fn(i32, *const u32)>("libGLESv2.so.2", "glDeleteBuffers");
    unsafe { real(n, buffers) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDeleteFramebuffers(n: i32, framebuffers: *const u32) {
    let real: unsafe extern "C" fn(i32, *const u32) = crate::dl::sym::<unsafe extern "C" fn(i32, *const u32)>("libGLESv2.so.2", "glDeleteFramebuffers");
    unsafe { real(n, framebuffers) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDeleteProgram(program: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glDeleteProgram");
    unsafe { real(program) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDeleteRenderbuffers(n: i32, renderbuffers: *const u32) {
    let real: unsafe extern "C" fn(i32, *const u32) = crate::dl::sym::<unsafe extern "C" fn(i32, *const u32)>("libGLESv2.so.2", "glDeleteRenderbuffers");
    unsafe { real(n, renderbuffers) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDeleteShader(shader: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glDeleteShader");
    unsafe { real(shader) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDeleteTextures(n: i32, textures: *const u32) {
    let real: unsafe extern "C" fn(i32, *const u32) = crate::dl::sym::<unsafe extern "C" fn(i32, *const u32)>("libGLESv2.so.2", "glDeleteTextures");
    unsafe { real(n, textures) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDepthFunc(func: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glDepthFunc");
    unsafe { real(func) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDepthMask(flag: u8) {
    let real: unsafe extern "C" fn(u8) = crate::dl::sym::<unsafe extern "C" fn(u8)>("libGLESv2.so.2", "glDepthMask");
    unsafe { real(flag) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDepthRangef(n: f32, f: f32) {
    let real: unsafe extern "C" fn(f32, f32) = crate::dl::sym::<unsafe extern "C" fn(f32, f32)>("libGLESv2.so.2", "glDepthRangef");
    unsafe { real(n, f) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDetachShader(program: u32, shader: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glDetachShader");
    unsafe { real(program, shader) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDisable(cap: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glDisable");
    unsafe { real(cap) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDisableVertexAttribArray(index: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glDisableVertexAttribArray");
    unsafe { real(index) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDrawArrays(mode: u32, first: i32, count: i32) {
    let real: unsafe extern "C" fn(u32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, i32)>("libGLESv2.so.2", "glDrawArrays");
    unsafe { real(mode, first, count) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glDrawElements(mode: u32, count: i32, type_: u32, indices: *const c_void) {
    let real: unsafe extern "C" fn(u32, i32, u32, *const c_void) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, u32, *const c_void)>("libGLESv2.so.2", "glDrawElements");
    unsafe { real(mode, count, type_, indices) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glEnable(cap: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glEnable");
    unsafe { real(cap) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glEnableVertexAttribArray(index: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glEnableVertexAttribArray");
    unsafe { real(index) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glFinish(void: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glFinish");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glFlush(void: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glFlush");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glFramebufferRenderbuffer(target: u32, attachment: u32, renderbuffertarget: u32, renderbuffer: u32) {
    let real: unsafe extern "C" fn(u32, u32, u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, u32, u32)>("libGLESv2.so.2", "glFramebufferRenderbuffer");
    unsafe { real(target, attachment, renderbuffertarget, renderbuffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glFramebufferTexture2D(target: u32, attachment: u32, textarget: u32, texture: u32, level: i32) {
    let real: unsafe extern "C" fn(u32, u32, u32, u32, i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, u32, u32, i32)>("libGLESv2.so.2", "glFramebufferTexture2D");
    unsafe { real(target, attachment, textarget, texture, level) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glFrontFace(mode: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glFrontFace");
    unsafe { real(mode) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGenBuffers(n: i32, buffers: *mut u32) {
    let real: unsafe extern "C" fn(i32, *mut u32) = crate::dl::sym::<unsafe extern "C" fn(i32, *mut u32)>("libGLESv2.so.2", "glGenBuffers");
    unsafe { real(n, buffers) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGenFramebuffers(n: i32, framebuffers: *mut u32) {
    let real: unsafe extern "C" fn(i32, *mut u32) = crate::dl::sym::<unsafe extern "C" fn(i32, *mut u32)>("libGLESv2.so.2", "glGenFramebuffers");
    unsafe { real(n, framebuffers) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGenRenderbuffers(n: i32, renderbuffers: *mut u32) {
    let real: unsafe extern "C" fn(i32, *mut u32) = crate::dl::sym::<unsafe extern "C" fn(i32, *mut u32)>("libGLESv2.so.2", "glGenRenderbuffers");
    unsafe { real(n, renderbuffers) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGenTextures(n: i32, textures: *mut u32) {
    let real: unsafe extern "C" fn(i32, *mut u32) = crate::dl::sym::<unsafe extern "C" fn(i32, *mut u32)>("libGLESv2.so.2", "glGenTextures");
    unsafe { real(n, textures) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGenerateMipmap(target: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glGenerateMipmap");
    unsafe { real(target) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetActiveAttrib(program: u32, index: u32, bufSize: i32, length: *mut i32, size: *mut i32, type_: *mut u32, name: *mut c_char) {
    let real: unsafe extern "C" fn(u32, u32, i32, *mut i32, *mut i32, *mut u32, *mut c_char) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, i32, *mut i32, *mut i32, *mut u32, *mut c_char)>("libGLESv2.so.2", "glGetActiveAttrib");
    unsafe { real(program, index, bufSize, length, size, type_, name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetActiveUniform(program: u32, index: u32, bufSize: i32, length: *mut i32, size: *mut i32, type_: *mut u32, name: *mut c_char) {
    let real: unsafe extern "C" fn(u32, u32, i32, *mut i32, *mut i32, *mut u32, *mut c_char) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, i32, *mut i32, *mut i32, *mut u32, *mut c_char)>("libGLESv2.so.2", "glGetActiveUniform");
    unsafe { real(program, index, bufSize, length, size, type_, name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetAttachedShaders(program: u32, maxCount: i32, count: *mut i32, shaders: *mut u32) {
    let real: unsafe extern "C" fn(u32, i32, *mut i32, *mut u32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, *mut i32, *mut u32)>("libGLESv2.so.2", "glGetAttachedShaders");
    unsafe { real(program, maxCount, count, shaders) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetAttribLocation(program: u32, name: *const c_char) -> i32 {
    let real: unsafe extern "C" fn(u32, *const c_char) -> i32 = crate::dl::sym::<unsafe extern "C" fn(u32, *const c_char) -> i32>("libGLESv2.so.2", "glGetAttribLocation");
    unsafe { real(program, name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetBooleanv(pname: u32, data: *mut u8) {
    let real: unsafe extern "C" fn(u32, *mut u8) = crate::dl::sym::<unsafe extern "C" fn(u32, *mut u8)>("libGLESv2.so.2", "glGetBooleanv");
    unsafe { real(pname, data) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetBufferParameteriv(target: u32, pname: u32, params: *mut i32) {
    let real: unsafe extern "C" fn(u32, u32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut i32)>("libGLESv2.so.2", "glGetBufferParameteriv");
    unsafe { real(target, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetError(void: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libGLESv2.so.2", "glGetError");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetFloatv(pname: u32, data: *mut f32) {
    let real: unsafe extern "C" fn(u32, *mut f32) = crate::dl::sym::<unsafe extern "C" fn(u32, *mut f32)>("libGLESv2.so.2", "glGetFloatv");
    unsafe { real(pname, data) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetFramebufferAttachmentParameteriv(target: u32, attachment: u32, pname: u32, params: *mut i32) {
    let real: unsafe extern "C" fn(u32, u32, u32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, u32, *mut i32)>("libGLESv2.so.2", "glGetFramebufferAttachmentParameteriv");
    unsafe { real(target, attachment, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetIntegerv(pname: u32, data: *mut i32) {
    let real: unsafe extern "C" fn(u32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, *mut i32)>("libGLESv2.so.2", "glGetIntegerv");
    unsafe { real(pname, data) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetProgramInfoLog(program: u32, bufSize: i32, length: *mut i32, infoLog: *mut c_char) {
    let real: unsafe extern "C" fn(u32, i32, *mut i32, *mut c_char) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, *mut i32, *mut c_char)>("libGLESv2.so.2", "glGetProgramInfoLog");
    unsafe { real(program, bufSize, length, infoLog) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetProgramiv(program: u32, pname: u32, params: *mut i32) {
    let real: unsafe extern "C" fn(u32, u32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut i32)>("libGLESv2.so.2", "glGetProgramiv");
    unsafe { real(program, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetRenderbufferParameteriv(target: u32, pname: u32, params: *mut i32) {
    let real: unsafe extern "C" fn(u32, u32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut i32)>("libGLESv2.so.2", "glGetRenderbufferParameteriv");
    unsafe { real(target, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetShaderInfoLog(shader: u32, bufSize: i32, length: *mut i32, infoLog: *mut c_char) {
    let real: unsafe extern "C" fn(u32, i32, *mut i32, *mut c_char) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, *mut i32, *mut c_char)>("libGLESv2.so.2", "glGetShaderInfoLog");
    unsafe { real(shader, bufSize, length, infoLog) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetShaderPrecisionFormat(shadertype: u32, precisiontype: u32, range: *mut i32, precision: *mut i32) {
    let real: unsafe extern "C" fn(u32, u32, *mut i32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut i32, *mut i32)>("libGLESv2.so.2", "glGetShaderPrecisionFormat");
    unsafe { real(shadertype, precisiontype, range, precision) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetShaderSource(shader: u32, bufSize: i32, length: *mut i32, source: *mut c_char) {
    let real: unsafe extern "C" fn(u32, i32, *mut i32, *mut c_char) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, *mut i32, *mut c_char)>("libGLESv2.so.2", "glGetShaderSource");
    unsafe { real(shader, bufSize, length, source) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetShaderiv(shader: u32, pname: u32, params: *mut i32) {
    let real: unsafe extern "C" fn(u32, u32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut i32)>("libGLESv2.so.2", "glGetShaderiv");
    unsafe { real(shader, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetString(name: u32) -> *const u8 {
    let real: unsafe extern "C" fn(u32) -> *const u8 = crate::dl::sym::<unsafe extern "C" fn(u32) -> *const u8>("libGLESv2.so.2", "glGetString");
    unsafe { real(name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetTexParameterfv(target: u32, pname: u32, params: *mut f32) {
    let real: unsafe extern "C" fn(u32, u32, *mut f32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut f32)>("libGLESv2.so.2", "glGetTexParameterfv");
    unsafe { real(target, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetTexParameteriv(target: u32, pname: u32, params: *mut i32) {
    let real: unsafe extern "C" fn(u32, u32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut i32)>("libGLESv2.so.2", "glGetTexParameteriv");
    unsafe { real(target, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetUniformLocation(program: u32, name: *const c_char) -> i32 {
    let real: unsafe extern "C" fn(u32, *const c_char) -> i32 = crate::dl::sym::<unsafe extern "C" fn(u32, *const c_char) -> i32>("libGLESv2.so.2", "glGetUniformLocation");
    unsafe { real(program, name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetUniformfv(program: u32, location: i32, params: *mut f32) {
    let real: unsafe extern "C" fn(u32, i32, *mut f32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, *mut f32)>("libGLESv2.so.2", "glGetUniformfv");
    unsafe { real(program, location, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetUniformiv(program: u32, location: i32, params: *mut i32) {
    let real: unsafe extern "C" fn(u32, i32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, *mut i32)>("libGLESv2.so.2", "glGetUniformiv");
    unsafe { real(program, location, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetVertexAttribPointerv(index: u32, pname: u32, pointer: *mut *mut c_void) {
    let real: unsafe extern "C" fn(u32, u32, *mut *mut c_void) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut *mut c_void)>("libGLESv2.so.2", "glGetVertexAttribPointerv");
    unsafe { real(index, pname, pointer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetVertexAttribfv(index: u32, pname: u32, params: *mut f32) {
    let real: unsafe extern "C" fn(u32, u32, *mut f32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut f32)>("libGLESv2.so.2", "glGetVertexAttribfv");
    unsafe { real(index, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glGetVertexAttribiv(index: u32, pname: u32, params: *mut i32) {
    let real: unsafe extern "C" fn(u32, u32, *mut i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *mut i32)>("libGLESv2.so.2", "glGetVertexAttribiv");
    unsafe { real(index, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glHint(target: u32, mode: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glHint");
    unsafe { real(target, mode) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glIsBuffer(buffer: u32) -> u8 {
    let real: unsafe extern "C" fn(u32) -> u8 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u8>("libGLESv2.so.2", "glIsBuffer");
    unsafe { real(buffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glIsEnabled(cap: u32) -> u8 {
    let real: unsafe extern "C" fn(u32) -> u8 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u8>("libGLESv2.so.2", "glIsEnabled");
    unsafe { real(cap) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glIsFramebuffer(framebuffer: u32) -> u8 {
    let real: unsafe extern "C" fn(u32) -> u8 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u8>("libGLESv2.so.2", "glIsFramebuffer");
    unsafe { real(framebuffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glIsProgram(program: u32) -> u8 {
    let real: unsafe extern "C" fn(u32) -> u8 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u8>("libGLESv2.so.2", "glIsProgram");
    unsafe { real(program) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glIsRenderbuffer(renderbuffer: u32) -> u8 {
    let real: unsafe extern "C" fn(u32) -> u8 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u8>("libGLESv2.so.2", "glIsRenderbuffer");
    unsafe { real(renderbuffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glIsShader(shader: u32) -> u8 {
    let real: unsafe extern "C" fn(u32) -> u8 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u8>("libGLESv2.so.2", "glIsShader");
    unsafe { real(shader) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glIsTexture(texture: u32) -> u8 {
    let real: unsafe extern "C" fn(u32) -> u8 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u8>("libGLESv2.so.2", "glIsTexture");
    unsafe { real(texture) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glLineWidth(width: f32) {
    let real: unsafe extern "C" fn(f32) = crate::dl::sym::<unsafe extern "C" fn(f32)>("libGLESv2.so.2", "glLineWidth");
    unsafe { real(width) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glLinkProgram(program: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glLinkProgram");
    unsafe { real(program) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glPixelStorei(pname: u32, param: i32) {
    let real: unsafe extern "C" fn(u32, i32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32)>("libGLESv2.so.2", "glPixelStorei");
    unsafe { real(pname, param) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glPolygonOffset(factor: f32, units: f32) {
    let real: unsafe extern "C" fn(f32, f32) = crate::dl::sym::<unsafe extern "C" fn(f32, f32)>("libGLESv2.so.2", "glPolygonOffset");
    unsafe { real(factor, units) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glReadPixels(x: i32, y: i32, width: i32, height: i32, format: u32, type_: u32, pixels: *mut c_void) {
    let real: unsafe extern "C" fn(i32, i32, i32, i32, u32, u32, *mut c_void) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, i32, i32, u32, u32, *mut c_void)>("libGLESv2.so.2", "glReadPixels");
    unsafe { real(x, y, width, height, format, type_, pixels) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glReleaseShaderCompiler(void: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glReleaseShaderCompiler");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glRenderbufferStorage(target: u32, internalformat: u32, width: i32, height: i32) {
    let real: unsafe extern "C" fn(u32, u32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, i32, i32)>("libGLESv2.so.2", "glRenderbufferStorage");
    unsafe { real(target, internalformat, width, height) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glSampleCoverage(value: f32, invert: u8) {
    let real: unsafe extern "C" fn(f32, u8) = crate::dl::sym::<unsafe extern "C" fn(f32, u8)>("libGLESv2.so.2", "glSampleCoverage");
    unsafe { real(value, invert) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glScissor(x: i32, y: i32, width: i32, height: i32) {
    let real: unsafe extern "C" fn(i32, i32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, i32, i32)>("libGLESv2.so.2", "glScissor");
    unsafe { real(x, y, width, height) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glShaderBinary(count: i32, shaders: *const u32, binaryFormat: u32, binary: *const c_void, length: i32) {
    let real: unsafe extern "C" fn(i32, *const u32, u32, *const c_void, i32) = crate::dl::sym::<unsafe extern "C" fn(i32, *const u32, u32, *const c_void, i32)>("libGLESv2.so.2", "glShaderBinary");
    unsafe { real(count, shaders, binaryFormat, binary, length) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glShaderSource(shader: u32, count: i32, string: *mut *const c_char, length: *const i32) {
    let real: unsafe extern "C" fn(u32, i32, *mut *const c_char, *const i32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, *mut *const c_char, *const i32)>("libGLESv2.so.2", "glShaderSource");
    unsafe { real(shader, count, string, length) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glStencilFunc(func: u32, ref_: i32, mask: u32) {
    let real: unsafe extern "C" fn(u32, i32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, u32)>("libGLESv2.so.2", "glStencilFunc");
    unsafe { real(func, ref_, mask) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glStencilFuncSeparate(face: u32, func: u32, ref_: i32, mask: u32) {
    let real: unsafe extern "C" fn(u32, u32, i32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, i32, u32)>("libGLESv2.so.2", "glStencilFuncSeparate");
    unsafe { real(face, func, ref_, mask) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glStencilMask(mask: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glStencilMask");
    unsafe { real(mask) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glStencilMaskSeparate(face: u32, mask: u32) {
    let real: unsafe extern "C" fn(u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32)>("libGLESv2.so.2", "glStencilMaskSeparate");
    unsafe { real(face, mask) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glStencilOp(fail: u32, zfail: u32, zpass: u32) {
    let real: unsafe extern "C" fn(u32, u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, u32)>("libGLESv2.so.2", "glStencilOp");
    unsafe { real(fail, zfail, zpass) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glStencilOpSeparate(face: u32, sfail: u32, dpfail: u32, dppass: u32) {
    let real: unsafe extern "C" fn(u32, u32, u32, u32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, u32, u32)>("libGLESv2.so.2", "glStencilOpSeparate");
    unsafe { real(face, sfail, dpfail, dppass) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glTexImage2D(target: u32, level: i32, internalformat: i32, width: i32, height: i32, border: i32, format: u32, type_: u32, pixels: *const c_void) {
    let real: unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void)>("libGLESv2.so.2", "glTexImage2D");
    unsafe { real(target, level, internalformat, width, height, border, format, type_, pixels) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glTexParameterf(target: u32, pname: u32, param: f32) {
    let real: unsafe extern "C" fn(u32, u32, f32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, f32)>("libGLESv2.so.2", "glTexParameterf");
    unsafe { real(target, pname, param) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glTexParameterfv(target: u32, pname: u32, params: *const f32) {
    let real: unsafe extern "C" fn(u32, u32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *const f32)>("libGLESv2.so.2", "glTexParameterfv");
    unsafe { real(target, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glTexParameteri(target: u32, pname: u32, param: i32) {
    let real: unsafe extern "C" fn(u32, u32, i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, i32)>("libGLESv2.so.2", "glTexParameteri");
    unsafe { real(target, pname, param) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glTexParameteriv(target: u32, pname: u32, params: *const i32) {
    let real: unsafe extern "C" fn(u32, u32, *const i32) = crate::dl::sym::<unsafe extern "C" fn(u32, u32, *const i32)>("libGLESv2.so.2", "glTexParameteriv");
    unsafe { real(target, pname, params) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glTexSubImage2D(target: u32, level: i32, xoffset: i32, yoffset: i32, width: i32, height: i32, format: u32, type_: u32, pixels: *const c_void) {
    let real: unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void)>("libGLESv2.so.2", "glTexSubImage2D");
    unsafe { real(target, level, xoffset, yoffset, width, height, format, type_, pixels) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform1f(location: i32, v0: f32) {
    let real: unsafe extern "C" fn(i32, f32) = crate::dl::sym::<unsafe extern "C" fn(i32, f32)>("libGLESv2.so.2", "glUniform1f");
    unsafe { real(location, v0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform1fv(location: i32, count: i32, value: *const f32) {
    let real: unsafe extern "C" fn(i32, i32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, *const f32)>("libGLESv2.so.2", "glUniform1fv");
    unsafe { real(location, count, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform1i(location: i32, v0: i32) {
    let real: unsafe extern "C" fn(i32, i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32)>("libGLESv2.so.2", "glUniform1i");
    unsafe { real(location, v0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform1iv(location: i32, count: i32, value: *const i32) {
    let real: unsafe extern "C" fn(i32, i32, *const i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, *const i32)>("libGLESv2.so.2", "glUniform1iv");
    unsafe { real(location, count, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform2f(location: i32, v0: f32, v1: f32) {
    let real: unsafe extern "C" fn(i32, f32, f32) = crate::dl::sym::<unsafe extern "C" fn(i32, f32, f32)>("libGLESv2.so.2", "glUniform2f");
    unsafe { real(location, v0, v1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform2fv(location: i32, count: i32, value: *const f32) {
    let real: unsafe extern "C" fn(i32, i32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, *const f32)>("libGLESv2.so.2", "glUniform2fv");
    unsafe { real(location, count, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform2i(location: i32, v0: i32, v1: i32) {
    let real: unsafe extern "C" fn(i32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, i32)>("libGLESv2.so.2", "glUniform2i");
    unsafe { real(location, v0, v1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform2iv(location: i32, count: i32, value: *const i32) {
    let real: unsafe extern "C" fn(i32, i32, *const i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, *const i32)>("libGLESv2.so.2", "glUniform2iv");
    unsafe { real(location, count, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform3f(location: i32, v0: f32, v1: f32, v2: f32) {
    let real: unsafe extern "C" fn(i32, f32, f32, f32) = crate::dl::sym::<unsafe extern "C" fn(i32, f32, f32, f32)>("libGLESv2.so.2", "glUniform3f");
    unsafe { real(location, v0, v1, v2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform3fv(location: i32, count: i32, value: *const f32) {
    let real: unsafe extern "C" fn(i32, i32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, *const f32)>("libGLESv2.so.2", "glUniform3fv");
    unsafe { real(location, count, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform3i(location: i32, v0: i32, v1: i32, v2: i32) {
    let real: unsafe extern "C" fn(i32, i32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, i32, i32)>("libGLESv2.so.2", "glUniform3i");
    unsafe { real(location, v0, v1, v2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform3iv(location: i32, count: i32, value: *const i32) {
    let real: unsafe extern "C" fn(i32, i32, *const i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, *const i32)>("libGLESv2.so.2", "glUniform3iv");
    unsafe { real(location, count, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform4f(location: i32, v0: f32, v1: f32, v2: f32, v3: f32) {
    let real: unsafe extern "C" fn(i32, f32, f32, f32, f32) = crate::dl::sym::<unsafe extern "C" fn(i32, f32, f32, f32, f32)>("libGLESv2.so.2", "glUniform4f");
    unsafe { real(location, v0, v1, v2, v3) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform4fv(location: i32, count: i32, value: *const f32) {
    let real: unsafe extern "C" fn(i32, i32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, *const f32)>("libGLESv2.so.2", "glUniform4fv");
    unsafe { real(location, count, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform4i(location: i32, v0: i32, v1: i32, v2: i32, v3: i32) {
    let real: unsafe extern "C" fn(i32, i32, i32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, i32, i32, i32)>("libGLESv2.so.2", "glUniform4i");
    unsafe { real(location, v0, v1, v2, v3) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniform4iv(location: i32, count: i32, value: *const i32) {
    let real: unsafe extern "C" fn(i32, i32, *const i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, *const i32)>("libGLESv2.so.2", "glUniform4iv");
    unsafe { real(location, count, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniformMatrix2fv(location: i32, count: i32, transpose: u8, value: *const f32) {
    let real: unsafe extern "C" fn(i32, i32, u8, *const f32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, u8, *const f32)>("libGLESv2.so.2", "glUniformMatrix2fv");
    unsafe { real(location, count, transpose, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniformMatrix3fv(location: i32, count: i32, transpose: u8, value: *const f32) {
    let real: unsafe extern "C" fn(i32, i32, u8, *const f32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, u8, *const f32)>("libGLESv2.so.2", "glUniformMatrix3fv");
    unsafe { real(location, count, transpose, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUniformMatrix4fv(location: i32, count: i32, transpose: u8, value: *const f32) {
    let real: unsafe extern "C" fn(i32, i32, u8, *const f32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, u8, *const f32)>("libGLESv2.so.2", "glUniformMatrix4fv");
    unsafe { real(location, count, transpose, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glUseProgram(program: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glUseProgram");
    unsafe { real(program) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glValidateProgram(program: u32) {
    let real: unsafe extern "C" fn(u32) = crate::dl::sym::<unsafe extern "C" fn(u32)>("libGLESv2.so.2", "glValidateProgram");
    unsafe { real(program) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttrib1f(index: u32, x: f32) {
    let real: unsafe extern "C" fn(u32, f32) = crate::dl::sym::<unsafe extern "C" fn(u32, f32)>("libGLESv2.so.2", "glVertexAttrib1f");
    unsafe { real(index, x) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttrib1fv(index: u32, v: *const f32) {
    let real: unsafe extern "C" fn(u32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(u32, *const f32)>("libGLESv2.so.2", "glVertexAttrib1fv");
    unsafe { real(index, v) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttrib2f(index: u32, x: f32, y: f32) {
    let real: unsafe extern "C" fn(u32, f32, f32) = crate::dl::sym::<unsafe extern "C" fn(u32, f32, f32)>("libGLESv2.so.2", "glVertexAttrib2f");
    unsafe { real(index, x, y) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttrib2fv(index: u32, v: *const f32) {
    let real: unsafe extern "C" fn(u32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(u32, *const f32)>("libGLESv2.so.2", "glVertexAttrib2fv");
    unsafe { real(index, v) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttrib3f(index: u32, x: f32, y: f32, z: f32) {
    let real: unsafe extern "C" fn(u32, f32, f32, f32) = crate::dl::sym::<unsafe extern "C" fn(u32, f32, f32, f32)>("libGLESv2.so.2", "glVertexAttrib3f");
    unsafe { real(index, x, y, z) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttrib3fv(index: u32, v: *const f32) {
    let real: unsafe extern "C" fn(u32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(u32, *const f32)>("libGLESv2.so.2", "glVertexAttrib3fv");
    unsafe { real(index, v) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttrib4f(index: u32, x: f32, y: f32, z: f32, w: f32) {
    let real: unsafe extern "C" fn(u32, f32, f32, f32, f32) = crate::dl::sym::<unsafe extern "C" fn(u32, f32, f32, f32, f32)>("libGLESv2.so.2", "glVertexAttrib4f");
    unsafe { real(index, x, y, z, w) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttrib4fv(index: u32, v: *const f32) {
    let real: unsafe extern "C" fn(u32, *const f32) = crate::dl::sym::<unsafe extern "C" fn(u32, *const f32)>("libGLESv2.so.2", "glVertexAttrib4fv");
    unsafe { real(index, v) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glVertexAttribPointer(index: u32, size: i32, type_: u32, normalized: u8, stride: i32, pointer: *const c_void) {
    let real: unsafe extern "C" fn(u32, i32, u32, u8, i32, *const c_void) = crate::dl::sym::<unsafe extern "C" fn(u32, i32, u32, u8, i32, *const c_void)>("libGLESv2.so.2", "glVertexAttribPointer");
    unsafe { real(index, size, type_, normalized, stride, pointer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn glViewport(x: i32, y: i32, width: i32, height: i32) {
    let real: unsafe extern "C" fn(i32, i32, i32, i32) = crate::dl::sym::<unsafe extern "C" fn(i32, i32, i32, i32)>("libGLESv2.so.2", "glViewport");
    unsafe { real(x, y, width, height) }
}

