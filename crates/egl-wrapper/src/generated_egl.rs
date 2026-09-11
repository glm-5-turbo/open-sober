// AUTO-GENERATED from Mesa headers (see gen_forward.py / scangl.py). Do not edit by hand.
// Each entry resolves the real Mesa symbol once and forwards the call.
#![allow(non_snake_case, unused_imports, clippy::missing_safety_doc)]
use std::ffi::{c_char, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglBindAPI(api: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libEGL.so.1", "eglBindAPI");
    unsafe { real(api) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglBindTexImage(dpy: *mut c_void, surface: *mut c_void, buffer: i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> u32>("libEGL.so.1", "eglBindTexImage");
    unsafe { real(dpy, surface, buffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglChooseConfig(dpy: *mut c_void, attrib_list: *const i32, configs: *mut *mut c_void, config_size: i32, num_config: *mut i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *const i32, *mut *mut c_void, i32, *mut i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *const i32, *mut *mut c_void, i32, *mut i32) -> u32>("libEGL.so.1", "eglChooseConfig");
    unsafe { real(dpy, attrib_list, configs, config_size, num_config) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglClientWaitSync(dpy: *mut c_void, sync: *mut c_void, flags: i32, timeout: u64) -> i32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32, u64) -> i32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32, u64) -> i32>("libEGL.so.1", "eglClientWaitSync");
    unsafe { real(dpy, sync, flags, timeout) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCopyBuffers(dpy: *mut c_void, surface: *mut c_void, target: isize) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, isize) -> u32>("libEGL.so.1", "eglCopyBuffers");
    unsafe { real(dpy, surface, target) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreateContext(dpy: *mut c_void, config: *mut c_void, share_context: *mut c_void, attrib_list: *const i32) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *const i32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *const i32) -> *mut c_void>("libEGL.so.1", "eglCreateContext");
    unsafe { real(dpy, config, share_context, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreateImage(dpy: *mut c_void, ctx: *mut c_void, target: u32, buffer: *mut c_void, attrib_list: *const isize) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut c_void, *const isize) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut c_void, *const isize) -> *mut c_void>("libEGL.so.1", "eglCreateImage");
    unsafe { real(dpy, ctx, target, buffer, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreatePbufferFromClientBuffer(dpy: *mut c_void, buftype: u32, buffer: *mut c_void, config: *mut c_void, attrib_list: *const i32) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void, *const i32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void, *const i32) -> *mut c_void>("libEGL.so.1", "eglCreatePbufferFromClientBuffer");
    unsafe { real(dpy, buftype, buffer, config, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreatePbufferSurface(dpy: *mut c_void, config: *mut c_void, attrib_list: *const i32) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, *const i32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, *const i32) -> *mut c_void>("libEGL.so.1", "eglCreatePbufferSurface");
    unsafe { real(dpy, config, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreatePixmapSurface(dpy: *mut c_void, config: *mut c_void, pixmap: isize, attrib_list: *const i32) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, isize, *const i32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, isize, *const i32) -> *mut c_void>("libEGL.so.1", "eglCreatePixmapSurface");
    unsafe { real(dpy, config, pixmap, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreatePlatformPixmapSurface(dpy: *mut c_void, config: *mut c_void, native_pixmap: *mut c_void, attrib_list: *const isize) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *const isize) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *const isize) -> *mut c_void>("libEGL.so.1", "eglCreatePlatformPixmapSurface");
    unsafe { real(dpy, config, native_pixmap, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreatePlatformWindowSurface(dpy: *mut c_void, config: *mut c_void, native_window: *mut c_void, attrib_list: *const isize) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *const isize) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *const isize) -> *mut c_void>("libEGL.so.1", "eglCreatePlatformWindowSurface");
    unsafe { real(dpy, config, native_window, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreateSync(dpy: *mut c_void, type_: u32, attrib_list: *const isize) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, u32, *const isize) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, u32, *const isize) -> *mut c_void>("libEGL.so.1", "eglCreateSync");
    unsafe { real(dpy, type_, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglCreateWindowSurface(dpy: *mut c_void, config: *mut c_void, win: isize, attrib_list: *const i32) -> *mut c_void {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, isize, *const i32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, isize, *const i32) -> *mut c_void>("libEGL.so.1", "eglCreateWindowSurface");
    unsafe { real(dpy, config, win, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglDestroyContext(dpy: *mut c_void, ctx: *mut c_void) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32>("libEGL.so.1", "eglDestroyContext");
    unsafe { real(dpy, ctx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglDestroyImage(dpy: *mut c_void, image: *mut c_void) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32>("libEGL.so.1", "eglDestroyImage");
    unsafe { real(dpy, image) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglDestroySurface(dpy: *mut c_void, surface: *mut c_void) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32>("libEGL.so.1", "eglDestroySurface");
    unsafe { real(dpy, surface) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglDestroySync(dpy: *mut c_void, sync: *mut c_void) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32>("libEGL.so.1", "eglDestroySync");
    unsafe { real(dpy, sync) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetConfigAttrib(dpy: *mut c_void, config: *mut c_void, attribute: i32, value: *mut i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *mut i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *mut i32) -> u32>("libEGL.so.1", "eglGetConfigAttrib");
    unsafe { real(dpy, config, attribute, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetConfigs(dpy: *mut c_void, configs: *mut *mut c_void, config_size: i32, num_config: *mut i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut *mut c_void, i32, *mut i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut *mut c_void, i32, *mut i32) -> u32>("libEGL.so.1", "eglGetConfigs");
    unsafe { real(dpy, configs, config_size, num_config) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetCurrentContext(void: u32) -> *mut c_void {
    let real: unsafe extern "C" fn(u32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(u32) -> *mut c_void>("libEGL.so.1", "eglGetCurrentContext");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetCurrentDisplay(void: u32) -> *mut c_void {
    let real: unsafe extern "C" fn(u32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(u32) -> *mut c_void>("libEGL.so.1", "eglGetCurrentDisplay");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetCurrentSurface(readdraw: i32) -> *mut c_void {
    let real: unsafe extern "C" fn(i32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(i32) -> *mut c_void>("libEGL.so.1", "eglGetCurrentSurface");
    unsafe { real(readdraw) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetDisplay(display_id: isize) -> *mut c_void {
    let real: unsafe extern "C" fn(isize) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(isize) -> *mut c_void>("libEGL.so.1", "eglGetDisplay");
    unsafe { real(display_id) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetError(void: u32) -> i32 {
    let real: unsafe extern "C" fn(u32) -> i32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> i32>("libEGL.so.1", "eglGetError");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetPlatformDisplay(platform: u32, native_display: *mut c_void, attrib_list: *const isize) -> *mut c_void {
    let real: unsafe extern "C" fn(u32, *mut c_void, *const isize) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(u32, *mut c_void, *const isize) -> *mut c_void>("libEGL.so.1", "eglGetPlatformDisplay");
    unsafe { real(platform, native_display, attrib_list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetProcAddress(procname: *const u32) -> *mut c_void {
    let real: unsafe extern "C" fn(*const u32) -> *mut c_void = crate::dl::sym::<unsafe extern "C" fn(*const u32) -> *mut c_void>("libEGL.so.1", "eglGetProcAddress");
    unsafe { real(procname) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglGetSyncAttrib(dpy: *mut c_void, sync: *mut c_void, attribute: i32, value: *mut isize) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *mut isize) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *mut isize) -> u32>("libEGL.so.1", "eglGetSyncAttrib");
    unsafe { real(dpy, sync, attribute, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglInitialize(dpy: *mut c_void, major: *mut i32, minor: *mut i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut i32, *mut i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut i32, *mut i32) -> u32>("libEGL.so.1", "eglInitialize");
    unsafe { real(dpy, major, minor) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglMakeCurrent(dpy: *mut c_void, draw: *mut c_void, read: *mut c_void, ctx: *mut c_void) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> u32>("libEGL.so.1", "eglMakeCurrent");
    unsafe { real(dpy, draw, read, ctx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglQueryAPI(void: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libEGL.so.1", "eglQueryAPI");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglQueryContext(dpy: *mut c_void, ctx: *mut c_void, attribute: i32, value: *mut i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *mut i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *mut i32) -> u32>("libEGL.so.1", "eglQueryContext");
    unsafe { real(dpy, ctx, attribute, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglQueryString(dpy: *mut c_void, name: i32) -> *const u32 {
    let real: unsafe extern "C" fn(*mut c_void, i32) -> *const u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, i32) -> *const u32>("libEGL.so.1", "eglQueryString");
    unsafe { real(dpy, name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglQuerySurface(dpy: *mut c_void, surface: *mut c_void, attribute: i32, value: *mut i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *mut i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *mut i32) -> u32>("libEGL.so.1", "eglQuerySurface");
    unsafe { real(dpy, surface, attribute, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglReleaseTexImage(dpy: *mut c_void, surface: *mut c_void, buffer: i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> u32>("libEGL.so.1", "eglReleaseTexImage");
    unsafe { real(dpy, surface, buffer) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglReleaseThread(void: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libEGL.so.1", "eglReleaseThread");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglSurfaceAttrib(dpy: *mut c_void, surface: *mut c_void, attribute: i32, value: i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32, i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32, i32) -> u32>("libEGL.so.1", "eglSurfaceAttrib");
    unsafe { real(dpy, surface, attribute, value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglSwapBuffers(dpy: *mut c_void, surface: *mut c_void) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32>("libEGL.so.1", "eglSwapBuffers");
    unsafe { real(dpy, surface) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglSwapInterval(dpy: *mut c_void, interval: i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, i32) -> u32>("libEGL.so.1", "eglSwapInterval");
    unsafe { real(dpy, interval) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglTerminate(dpy: *mut c_void) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void) -> u32>("libEGL.so.1", "eglTerminate");
    unsafe { real(dpy) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglWaitClient(void: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libEGL.so.1", "eglWaitClient");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglWaitGL(void: u32) -> u32 {
    let real: unsafe extern "C" fn(u32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(u32) -> u32>("libEGL.so.1", "eglWaitGL");
    unsafe { real(void) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglWaitNative(engine: i32) -> u32 {
    let real: unsafe extern "C" fn(i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(i32) -> u32>("libEGL.so.1", "eglWaitNative");
    unsafe { real(engine) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn eglWaitSync(dpy: *mut c_void, sync: *mut c_void, flags: i32) -> u32 {
    let real: unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> u32 = crate::dl::sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> u32>("libEGL.so.1", "eglWaitSync");
    unsafe { real(dpy, sync, flags) }
}

