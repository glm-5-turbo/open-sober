# Graphics Translation Layer — Research Findings & Recommendation

## Executive Summary

The original Sober binary (`sober`, 7.1MB Rust binary) uses Mesa's system `libEGL.so.1` and `libGLESv2.so.2` as a **pass-through** — it does NOT implement its own GLES-to-Vulkan translation layer. Instead, it relies on the system Mesa EGL/GLES libraries, which on modern desktops can be backed by the **zink** Gallium driver (GLES on top of Vulkan). The ARCHITECTURE.md from sober-oss mentions volk/detex as part of the pipeline, but the actual binary only links against the Mesa libEGL/libGLESv2 and resolves GLES function pointers dynamically. This report recommends adopting Mesa+zink as the default path and describes what stub libraries and texture conversion code are needed.

---

## 1. Recommended Approach: Mesa EGL/GLESv2 + zink (GLES→Vulkan)

### 1.1 Architecture

```
Roblox Android APK (ARM64)
       |
       | binary translation (arm64 → x86_64)
       | syscall mapping
       v
Roblox x86_64 binary (expects Android EGL + GLES 2.0/3.0)
       |
       | calls eglGetDisplay, eglCreateWindowSurface, glDrawElements, etc.
       v
libEGL.so.1  +  libGLESv2.so.2   (Mesa system libraries)
       |
       | Mesa's egl_dri2 driver
       |   → if MESA_LOADER_DRIVER_OVERRIDE=zink:
       |
       v
zink Gallium driver (Mesa built-in)
       |
       | translates OpenGL ES to Vulkan
       v
volk (dynamic Vulkan loader)
       |
       v
Desktop Vulkan Driver (NVIDIA RTX 3060 / Mesa Intel iGPU)
```

### 1.2 Why This Works

- **Mesa provides libEGL.so.1 and libGLESv2.so.2** which match the ABI that the Roblox Android binary expects when running under the translation layer.
- **zink is a Mesa Gallium driver** that transparently translates OpenGL (including GLES 2.0, 3.0, 3.1, and 3.2) to Vulkan. It is built into Mesa when configured with `-Dgallium-drivers=zink`. The Mesa driver feature matrix confirms zink implements **GLES 3.1** and **GLES 3.2** directly — not just desktop GL passthrough.
- **Mesa's EGL implementation supports both Wayland and X11** natively via the `egl_dri2` driver, which can be backed by any Gallium driver including zink.
- **No custom GLES→Vulkan translation code is needed** in the Open Sober project. The entire Mesa stack can be reused.

### 1.3 How to Enable zink

```bash
# Option A: Set environment variable before launching Roblox
export MESA_LOADER_DRIVER_OVERRIDE=zink

# Option B: Verify zink is available
MESA_LOADER_DRIVER_OVERRIDE=zink glxinfo | grep "OpenGL renderer"
# Should say: "Mesa Zink Driver" or similar

# Option C: On Ubuntu/Debian, zink is included in recent Mesa (24.x+)
# sudo apt install mesa-utils
# libEGL and libGLESv2 are part of libegl1 and libgles2
```

### 1.4 zink Support for OpenGL ES

The zink driver supports the full desktop OpenGL version ladder from 2.1 through 4.6. Since **Mesa's libGLESv2.so.2 implements OpenGL ES 2.0 and 3.0 on top of desktop OpenGL**, and zink translates that to Vulkan, the chain is:

```
GLES 2.0/3.0 call → Mesa GLES→GL translation → zink GL→Vulkan translation → Vulkan driver
```

This means GLES 2.0 and 3.0 are fully functional. The zink documentation lists OpenGL 2.1 as minimum (requires Vulkan 1.0), so GLES 2.0 (which maps to GLES2 contexts backed by GL 2.x/3.x) works. GLES 3.0 requires OpenGL 3.x features which zink supports starting from Vulkan 1.0 with specific extensions.

### 1.5 Key Environment Variables

| Variable | Purpose |
|---|---|
| `MESA_LOADER_DRIVER_OVERRIDE=zink` | Force Mesa to use zink as the Gallium driver |
| `EGL_PLATFORM=wayland` or `EGL_PLATFORM=x11` | Select display platform at runtime |
| `LIBGL_ALWAYS_SOFTWARE=1` | Fallback to software rendering (useful for debugging) |
| `ZINK_DEBUG=validation` | Enable Vulkan validation layers via zink |
| `ZINK_DESCRIPTORS=auto` | zink descriptor management mode |
| `MESA_GLES_VERSION_OVERRIDE=3.0` | Force GLES context version if needed |
| `EGL_LOG_LEVEL=debug` | Enable EGL debug logging |

### 1.6 Limitations

- **zink requires Vulkan 1.0+** — works on both NVIDIA (RTX 3060) and Intel iGPU (Mesa Vulkan driver).
- **Performance overhead**: zink adds translation overhead compared to native Vulkan, but for a game like Roblox this is acceptable.
- **Feature parity**: Some niche GLES extensions may not be fully implemented in zink. The Roblox Android APK uses a relatively standard subset (see Section 5).
- **Mesa version**: Requires Mesa 24.0+ for good zink support. Mesa 24.3+ is recommended.

---

## 2. Alternative: ANGLE (GLES→Vulkan)

### 2.1 What ANGLE Provides

ANGLE (Almost Native Graphics Layer Engine) is Google's GLES implementation that translates to native backends including Vulkan. It provides:

- **libEGL.so** — implementing EGL 1.5
- **libGLESv2.so** — implementing GLES 2.0, 3.0, 3.1 (3.2 in progress)
- **Vulkan backend** — fully conformant on Linux
- Also includes an experimental OpenCL runtime

### 2.2 How to Use ANGLE Standalone

```bash
# Build ANGLE on Linux
git clone https://chromium.googlesource.com/angle/angle
cd angle
# Install depot_tools, fetch angle
gn gen out/Release --args='is_debug=false angle_enable_vulkan=true angle_enable_gl=false angle_enable_d3d9=false angle_enable_d3d11=false angle_enable_metal=false'
autoninja -C out/Release
# Output: libEGL.so, libGLESv2.so in out/Release/
```

Then deploy these alongside the Roblox binary, ensuring LD_LIBRARY_PATH points to them. The Vulkan backend is selected at EGL init via the `EGL_ANGLE_platform_angle` extension:

```c
// At EGL initialization:
EGLDisplay display = eglGetPlatformDisplayEXT(
    EGL_PLATFORM_ANGLE_ANGLE,
    EGL_DEFAULT_DISPLAY,
    attribs  // with EGL_PLATFORM_ANGLE_TYPE_ANGLE = EGL_PLATFORM_ANGLE_TYPE_VULKAN_ANGLE
);
```

### 2.3 ANGLE vs Mesa+zink Comparison

| Aspect | Mesa+zink | ANGLE |
|---|---|---|
| **Deployment** | Included in system Mesa (`libegl1`, `libgles2`) | Must build from source + depot_tools (Chromium build system) |
| **Build complexity** | None (system library) | High (Chromium GN/Ninja, 100k+ files) |
| **GLES version** | 2.0, 3.0 (via Mesa GLES→GL) | 2.0, 3.0, 3.1, 3.2 in progress |
| **Vulkan backend** | Via zink (GL→zink→Vulkan) | Direct native Vulkan backend |
| **Performance** | Double translation (GLES→GL→Vulkan) | Direct (GLES→Vulkan) |
| **Wayland support** | Yes (Mesa egl_dri2) | Yes |
| **X11 support** | Yes | Yes |
| **Licensing** | MIT (Mesa) | BSD (ANGLE) |
| **Maintenance** | Handled by Mesa community | Handled by Google/Chromium |

### 2.4 Recommendation

**Use Mesa+zink for the initial MVP**. It requires zero custom build steps and is available on any modern Linux distribution. If performance with zink is insufficient, ANGLE can be added as a higher-performance alternative (direct GLES→Vulkan without the GL intermediate).

---

## 3. Writing Stub libEGL.so and libGLESv2.so

### 3.1 Why Stubs May Be Needed

The Roblox Android APK calls EGL and GLES functions. Under binary translation (ARM64→x86-64), these calls become x86-64 function calls to the expected library sonames:

- `libEGL.so` (Android expects `libEGL.so`, on desktop it's `libEGL.so.1`)
- `libGLESv2.so` (Android expects `libGLESv2.so`, on desktop it's `libGLESv2.so.2`)

The binary translator must either:
1. **DLOPEN** the system Mesa libraries with the correct soname, OR
2. **Provide stub .so files** that forward calls to Mesa

### 3.2 Approach: LD_PRELOAD Stubs

The simplest approach is to create thin stub libraries that:
1. Export all the EGL/GLES entry points the app uses
2. dlopen the real Mesa libEGL.so.1 / libGLESv2.so.2
3. Forward each call

#### EGL Surface Area (from sober-oss analysis)

The Roblox Android binary calls these EGL functions (dynamically resolved via `eglGetProcAddress` or direct linking):

- `eglGetError` (confirmed in strings)

The Android EGL init sequence a typical app follows:
1. `eglGetDisplay(EGL_DEFAULT_DISPLAY)`
2. `eglInitialize(display, &major, &minor)`
3. `eglChooseConfig(display, attribs, &config, ...)`
4. `eglCreateWindowSurface(display, config, native_window, attribs)`
5. `eglCreateContext(display, config, share_context, attribs)`
6. `eglMakeCurrent(display, surface, surface, context)`
7. Render loop: `eglSwapBuffers(display, surface)`
8. `eglDestroySurface`, `eglDestroyContext`, `eglTerminate`

A complete stub libEGL.so must export all EGL 1.4/1.5 functions plus common extensions.

#### GLES Surface Area (confirmed from sober strings)

The 90+ GLES functions used by Roblox:

```
glActiveTexture         glAttachShader          glBindAttribLocation
glBindBuffer            glBindFramebuffer       glBindRenderbuffer
glBindTexture           glBlendColor            glBlendEquation
glBlendEquationSeparate glBlendFunc             glBlendFuncSeparate
glBufferData            glBufferSubData         glCheckFramebufferStatus
glClear                 glClearColor            glClearDepthf
glClearStencil          glColorMask             glCompileShader
glCompressedTexImage2D  glCompressedTexSubImage2D
glCopyTexSubImage2D     glCreateProgram         glCreateShader
glCullFace              glDeleteBuffers         glDeleteFramebuffers
glDeleteProgram         glDeleteRenderbuffers   glDeleteShader
glDeleteTextures        glDepthFunc             glDepthMask
glDetachShader          glDisable               glDisableVertexAttribArray
glDrawArrays            glDrawElements          glEnable
glEnableVertexAttribArray glFlush               glFramebufferRenderbuffer
glFramebufferTexture2D  glFrontFace             glGenBuffers
glGenerateMipmap        glGenFramebuffers       glGenRenderbuffers
glGenTextures           glGetActiveAttrib       glGetActiveUniform
glGetAttribLocation     glGetError              glGetFloatv
glGetIntegerv           glGetProgramInfoLog     glGetProgramiv
glGetShaderInfoLog      glGetShaderiv           glGetShaderPrecisionFormat
glGetString             glGetUniformLocation    glGetVertexAttribiv
glGetVertexAttribPointerv glIsEnabled           glIsProgram
glLinkProgram           glPixelStorei           glPolygonOffset
glReadPixels            glReleaseShaderCompiler glRenderbufferStorage
glScissor               glShaderSource          glStencilFunc
glStencilFuncSeparate   glStencilMask           glStencilOp
glStencilOpSeparate     glTexImage2D            glTexParameterf
glTexParameterfv        glTexParameteri         glTexSubImage2D
glUniform1i             glUniform1iv            glUniform4fv
glUniformMatrix3fv      glUniformMatrix4fv      glUseProgram
glVertexAttribPointer   glViewport
```

Note: `glCompressedTexImage2D` and `glCompressedTexSubImage2D` are present — these are the texture format conversion hooks.

### 3.3 Stub Implementation Strategy

Since the Open Sober project is Rust-based, the stubs must be implemented as **C-compatible shared libraries** (.so files that export C symbols). Options:

**Option A: C stubs (simplest)**
```c
// stub_egl.c — compile to libEGL.so
#include <EGL/egl.h>
#include <dlfcn.h>

static void *real_egl = NULL;
static void init() {
    if (!real_egl) real_egl = dlopen("libEGL.so.1", RTLD_LAZY | RTLD_LOCAL);
}

EGLDisplay eglGetDisplay(EGLNativeDisplayType display_id) {
    init();
    static EGLDisplay (*real)(EGLNativeDisplayType) = NULL;
    if (!real) real = dlsym(real_egl, "eglGetDisplay");
    return real(display_id);
}
// ... one forwarding function per EGL API
```

**Option B: Rust stubs (if Rust FFI is preferred)**
```rust
// stub_egl.rs — compile to cdylib producing libEGL.so
#[no_mangle]
pub extern "C" fn eglGetDisplay(display_id: EGLNativeDisplayType) -> EGLDisplay {
    // Same pattern: dlopen, dlsym, call
}
```

**Option C: LD_PRELOAD of Mesa's real libraries**
- If the binary translator maps `libEGL.so` → `libEGL.so.1`, no stubs are needed.
- The Flatpak manifest from the original Sober shows: `--socket=wayland --socket=fallback-x11 --device=dri` with NO bundled libEGL/libGLESv2 — it relies entirely on the GNOME runtime's Mesa libraries. No stubs are built.

**Recommendation**: Start with Option C (symlink or dlopen the real Mesa libs). Only build stubs if there's an ABI mismatch.

---

## 4. Texture Format Conversion

### 4.1 Android Texture Formats

Android OpenGL ES 2.0/3.0 supports these compressed texture formats:

| Format | GL Enum | GLES Version | Used by Roblox |
|---|---|---|---|
| ETC1 | `GL_ETC1_RGB8_OES` (0x8D64) | ES 2.0 (extension) | Yes |
| ETC2 RGB | `GL_COMPRESSED_RGB8_ETC2` (0x9274) | ES 3.0+ | Yes |
| ETC2 RGBA1 | `GL_COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2` (0x9276) | ES 3.0+ | Yes |
| ETC2 RGBA8 | `GL_COMPRESSED_RGBA8_ETC2_EAC` (0x9278) | ES 3.0+ | Yes |
| EAC R (unsigned) | `GL_COMPRESSED_R11_EAC` (0x9270) | ES 3.0+ | Yes |
| EAC RG (unsigned) | `GL_COMPRESSED_RG11_EAC` (0x9272) | ES 3.0+ | Yes |
| ASTC (various) | `GL_COMPRESSED_RGBA_ASTC_*` (0x93B0–0x93BF) | ES 3.2+ / extension | Yes |
| ATC | `GL_ATC_RGB_AMD` / `GL_ATC_RGBA_AMD` | Adreno extension | Possibly (legacy) |
| PVRTC | `GL_COMPRESSED_RGB_PVRTC_*` | PowerVR extension | Unlikely |

### 4.2 Desktop GL Texture Support Gap

Desktop OpenGL does NOT support:
- **ETC1/ETC2/EAC** — not in core desktop GL (only GLES)
- **ASTC** — not supported on NVIDIA GPUs (only Intel/ARM)
- **ATC** — Qualcomm-specific

Desktop OpenGL DOES support:
- **BC1-BC7 (DXTn)** — universally supported (S3TC/BPTC)
- **RGBA8 uncompressed** — always works

### 4.3 The Texture Conversion Pipeline

When Roblox calls `glCompressedTexImage2D` with an Android format, we must:

1. **Intercept** the call
2. **Decompress** the Android format to raw RGBA pixels
3. **Upload** as uncompressed `GL_RGBA8` or re-compress as BCn

### 4.4 Recommended: texture2ddecoder crate

The `texture2ddecoder` (v0.1.2, MIT/Apache-2.0) is a pure-Rust, no-std texture decoder supporting every format Android uses:

- **ETC1**: `decode_etc1()`
- **ETC2 RGB**: `decode_etc2_rgb()`
- **ETC2 RGBA1**: `decode_etc2_rgba1()`
- **ETC2 RGBA8**: `decode_etc2_rgba8()`
- **EAC R (signed/unsigned)**: `decode_eacr()`, `decode_eacr_signed()`
- **EAC RG (signed/unsigned)**: `decode_eacrg()`, `decode_eacrg_signed()`
- **ASTC** (all block sizes): `decode_astc()`, `decode_astc_4_4()` etc.
- **ATC**: `decode_atc_rgb4()`, `decode_atc_rgba8()`
- **PVRTC**: `decode_pvrtc()`
- **BC1-BC7**: `decode_bc1()` through `decode_bc7()`
- **Crunch**: `decode_crunch()`, `decode_unity_crunch()` (Roblox uses Unity, so this is important)

**Output format**: All functions output `&[u32]` (BGRA pixels) or take `&mut [u32]` as destination.

The crate has 66k+ downloads, is actively maintained (latest: 2025-03-21), and is no-std compatible.

### 4.5 Intercepting glCompressedTexImage2D

In our stub `libGLESv2.so`, override:

```c
void glCompressedTexImage2D(GLenum target, GLint level, GLenum internalformat,
                            GLsizei width, GLsizei height, GLint border,
                            GLsizei imageSize, const void *data) {
    if (internalformat is an Android-specific format) {
        // Decompress to RGBA using texture2ddecoder
        uint32_t *rgba = decode_etc2_rgba8(data, width, height);
        // Upload as uncompressed GL_RGBA8
        glTexImage2D(target, level, GL_RGBA8, width, height, border,
                     GL_RGBA, GL_UNSIGNED_BYTE, rgba);
        free(rgba);
    } else {
        real_glCompressedTexImage2D(target, level, internalformat,
                                     width, height, border, imageSize, data);
    }
}
```

For better GPU memory efficiency, consider re-compressing as BC3/BC7 with a fast GPU compressor (e.g., `etc2comp`, `crunch`) but start with uncompressed RGBA upload.

### 4.6 Google etc2comp

Google's `etc2comp` (https://github.com/google/etc2comp) provides ETC compression/decompression. It is a C++ library with an Apache 2.0 license. However, `texture2ddecoder` is a simpler Rust-native choice for Open Sober since the project is already Rust-based.

---

## 5. Native Window System Abstraction

### 5.1 Android EGL Native Window Types

Android uses `ANativeWindow` (from `libandroid.so`) as the EGL native window type. On desktop, Mesa's EGL uses:

- **Wayland**: `struct wl_surface*`
- **X11**: `Window` (Xlib `XID`)
- **DRM/KMS**: `gbm_surface` (via libgbm)

### 5.2 The Bridge

The binary translator must convert the Android `ANativeWindow` handle into a desktop-native window handle. This likely works as follows in the original Sober:

1. Sober creates a desktop window (via Wayland/X11)
2. The binary translator maps Android's `ANativeWindow` calls to the desktop window
3. When Roblox calls `eglCreateWindowSurface(display, config, native_window, attribs)`:
   - `native_window` is actually an `ANativeWindow*`
   - The translator has stored the native window handle behind it
   - The EGL layer needs to pass the correct desktop-native handle to Mesa's `eglCreateWindowSurface`

### 5.3 Implementation Approach

The simplest approach for the MVP:

```rust
// In the translator, when Roblox creates a native window:
struct NativeWindowBridge {
    desktop_window_handle: *mut c_void,  // wl_surface* or X11 Window
    width: u32,
    height: u32,
}

// Map the Android ANativeWindow pointer to a bridge struct
// The EGL stub retrieves this mapping when eglCreateWindowSurface is called
```

For Wayland specifically, `libwayland-egl.so.1` provides `wl_egl_window_create()` and `wl_egl_window_resize()` which create EGL-compatible native window handles from `wl_surface*`.

### 5.4 Platform Support

| Platform | Mesa Platform String | EGL Platform | Status |
|---|---|---|---|
| Wayland | `wayland` | `EGL_PLATFORM_WAYLAND_EXT` | Primary target (preferred) |
| X11 | `x11` | `EGL_PLATFORM_X11_EXT` | Fallback |
| Surfaceless | `surfaceless` | — | Headless/test |

From the Sober Flatpak manifest: `--socket=wayland --socket=fallback-x11` — both are supported, Wayland preferred.

---

## 6. Input Event Mapping (Android → Desktop)

### 6.1 What Roblox Android Expects

Roblox on Android expects:
- **Touch events** (ACTION_DOWN, ACTION_MOVE, ACTION_UP, ACTION_POINTER_DOWN, etc.)
- **Multi-touch** (pinch-to-zoom, etc.)
- **Sensor events** (accelerometer for device orientation)
- **Keyboard** (for chat)
- **Game controller** (bluetooth controller support)

### 6.2 Desktop Input Mapping Strategy

| Android Input | Desktop Source | Mapping |
|---|---|---|
| Single touch (ACTION_DOWN/UP) | Mouse click | Left mouse button → touch at cursor position |
| Touch move (ACTION_MOVE) | Mouse move (button held) | Track mouse movement while left button held |
| Multi-touch (pinch) | Mouse scroll wheel | Scroll → simulated two-finger pinch |
| Multi-touch (two-finger) | Shift+click or right-click | Second touch point |
| Keyboard (chat) | Physical keyboard | Direct key event forwarding |
| Accelerometer | Disabled / fixed orientation | Report device as landscape |
| Gyroscope | Disabled | Report zero rotation |
| Game controller | SDL2 gamepad events | Map to Android keycodes |

### 6.3 Implementation via Wayland/X11

The original Sober uses `libudev` (confirmed in strings: `events-udev` feature) for input device handling on Wayland. It also links:
- `libXi.so.6` — X11 Input extension
- `libXtst.so.6` — X11 XTEST extension
- `libXcursor.so.1` — cursor management
- `libwayland-cursor.so.0` — Wayland cursor

For the MVP, the simplest input model is:

```rust
// 1. Create a window with SDL2 or raw X11/Wayland
// 2. Listen for mouse events
// 3. Forward to Roblox as Android touch events via the syscall/API mapping layer

struct InputMapper {
    touch_active: bool,
    touch_x: f32,
    touch_y: f32,
    // Map mouse position relative to window
}

// SDL2 approach:
sdl2::event::poll() {
    Event::MouseButtonDown { x, y, .. } => {
        send_touch_event(ACTION_DOWN, x, window_height - y);  // Android has inverted Y
    }
    Event::MouseMotion { x, y, .. } => {
        send_touch_event(ACTION_MOVE, x, window_height - y);
    }
    Event::MouseWheel { y, .. } => {
        // Two-finger scroll simulation
        send_touch_event(ACTION_POINTER_DOWN, center_x, center_y, pointer_id=2);
        send_touch_event(ACTION_MOVE, center_x, center_y + y * scroll_factor);
        send_touch_event(ACTION_POINTER_UP, center_x, center_y + y * scroll_factor, pointer_id=2);
    }
}
```

---

## 7. The Role of volk

### 7.1 What volk Provides

volk (github.com/zeux/volk, MIT license) is a **meta-loader for the Vulkan API**. It dynamically loads all Vulkan entry points at runtime, avoiding the need to link against the system Vulkan loader.

**Key API**:
```c
VkResult volkInitialize();                              // Load system Vulkan loader
void volkLoadInstance(VkInstance instance);              // Load all instance + device entrypoints
void volkLoadInstanceOnly(VkInstance instance);          // Load instance-level only
void volkLoadDevice(VkDevice device);                    // Load device functions directly (skip loader)
void volkLoadDeviceTable(VolkDeviceTable* table, ...);   // For multi-device scenarios
```

### 7.2 How Sober Uses volk

If we take the Mesa+zink path, zink itself links against the system Vulkan loader (`libvulkan.so.1`), which handles all Vulkan function resolution internally. **volk is not needed** at the Open Sober application level when using Mesa+zink.

volk would only be needed if Open Sober implements its own Vulkan renderer directly (bypassing Mesa). In that case:

1. Include `volk.h` (instead of `vulkan.h`)
2. Call `volkInitialize()` early
3. Create Vulkan instance, call `volkLoadInstance(instance)`
4. Create device, optionally call `volkLoadDevice(device)` for direct dispatch

**However**: The sovereign-oss ARCHITECTURE.md mentions volk in the rendering pipeline. This may refer to zink's internal use of volk, or the original Sober may use a custom Vulkan backend that hasn't been fully reverse-engineered yet. The binary import table shows NO direct volk dependency — only `libEGL.so.1` and `libGLESv2.so.2`.

### 7.3 Recommendation

Do NOT build against volk directly in the Open Sober project at this stage. Let Mesa/zink handle Vulkan loading. Add volk as a dependency only if we implement a custom Vulkan renderer in the future.

---

## 8. Texture Format Deep Dive (ETC2, ASTC, and Desktop Support)

### 8.1 ETC2 Support on Desktop

Desktop GPUs generally do NOT support ETC2 natively:
- **NVIDIA RTX 3060**: No ETC2 hardware decode (desktop NVIDIA doesn't support it)
- **Mesa Intel iGPU**: ETC2 is supported in the Vulkan driver (Intel ANV), but NOT in desktop OpenGL
- **AMD**: Does not support ETC2 in desktop OpenGL

However, Mesa includes a **software fallback** for ETC2 textures. When a GLES app uploads an ETC2 texture via `glCompressedTexImage2D`, the Mesa GLES library decompresses it to RGBA in software before uploading to the GPU. This is transparent to the application.

**This means texture decompression may already work automatically through Mesa**. If the Mesa GLES library handles the decompression, the texture2ddecoder approach is a fallback if Mesa's software decoder is too slow.

### 8.2 ASTC on Desktop

ASTC (Adaptive Scalable Texture Compression) is NOT supported by NVIDIA GPUs at all (only Intel ARC+ and ARM). Mesa may or may not have a software fallback for ASTC in the GLES library. If Roblox uses ASTC textures, we will likely need `texture2ddecoder`'s `decode_astc()` for ASTC→RGBA conversion.

### 8.3 Recommended Texture Pipeline

```
glCompressedTexImage2D(..., GL_ETC2_RGB8, ..., data)
    │
    ├─ Let Mesa handle it → Mesa decompresses to RGBA → works automatically
    │  (best path for MVP, may have performance impact)
    │
    └─ Custom interception:
       ┌─────────────────────────────────┐
       │ texture2ddecoder::decode_etc2() │ → RGBA pixels
       │ glTexImage2D(GL_RGBA, ...)      │ → upload to GPU
       └─────────────────────────────────┘
```

For the MVP, let Mesa handle decompression transparently. Add explicit texture2ddecoder-based interception ONLY if:
1. Mesa fails to decompress a format (e.g., ASTC)
2. Performance of software decompression is too slow
3. You need to re-compress as BCn for GPU memory savings

---

## 9. Minimum Viable Graphics Wrapper — Build Plan

### Phase 1: EGL/GLES Stub Library (libegl-wrapper + libglesv2-wrapper)

Create two new crates in the workspace:

```toml
# crates/egl-wrapper/Cargo.toml
[package]
name = "egl-wrapper"
[lib]
crate-type = ["cdylib"]
name = "EGL"  # produces libEGL.so

[dependencies]
libc = "0.2"
texture2ddecoder = "0.1"
```

```toml
# crates/glesv2-wrapper/Cargo.toml
[package]
name = "glesv2-wrapper"
[lib]
crate-type = ["cdylib"]
name = "GLESv2"  # produces libGLESv2.so

[dependencies]
libc = "0.2"
texture2ddecoder = "0.1"
```

#### Stub Implementation (C FFI in Rust)

```rust
// crates/egl-wrapper/src/lib.rs
use std::ffi::{CStr, CString};
use libc::{c_void, c_int, c_uint, c_char};

// Load real Mesa EGL
lazy_static! {
    static ref REAL_EGL: Library = unsafe {
        Library::new("libEGL.so.1").expect("Mesa libEGL.so.1 not found")
    };
}

macro_rules! forward {
    ($name:ident($($param:ident: $t:ty),*) -> $ret:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name($($param: $t),*) -> $ret {
            type F = unsafe extern "C" fn($($t),*) -> $ret;
            let func: F = std::mem::transmute(
                REAL_EGL.get::<extern "C" fn()>(stringify!($name).as_bytes())
                    .expect(concat!("Missing ", stringify!($name)))
            );
            func($($param),*)
        }
    };
}

forward!(eglGetDisplay(display_id: EGLNativeDisplayType) -> EGLDisplay);
forward!(eglInitialize(dpy: EGLDisplay, major: *mut EGLint, minor: *mut EGLint) -> EGLBoolean);
forward!(eglChooseConfig(dpy: EGLDisplay, attribs: *const EGLint, configs: *mut EGLConfig, config_size: EGLint, num_config: *mut EGLint) -> EGLBoolean);
forward!(eglCreateWindowSurface(dpy: EGLDisplay, config: EGLConfig, win: EGLNativeWindowType, attribs: *const EGLint) -> EGLSurface);
forward!(eglCreateContext(dpy: EGLDisplay, config: EGLConfig, share: EGLContext, attribs: *const EGLint) -> EGLContext);
forward!(eglMakeCurrent(dpy: EGLDisplay, draw: EGLSurface, read: EGLSurface, ctx: EGLContext) -> EGLBoolean);
forward!(eglSwapBuffers(dpy: EGLDisplay, surface: EGLSurface) -> EGLBoolean);
forward!(eglGetError() -> EGLint);
forward!(eglGetProcAddress(procname: *const c_char) -> *mut c_void);
// ... one line per EGL 1.4 function (~35 total)
```

For `glCompressedTexImage2D` in the GLES stub, add texture decompression logic:

```rust
// crates/glesv2-wrapper/src/texture_decompress.rs
use texture2ddecoder as dec;
use libc::{c_void, c_uint, c_int};

pub fn decompress_compressed_tex_image(
    internalformat: GLenum,
    width: usize,
    height: usize,
    data: &[u8],
) -> Option<Vec<u32>> {
    let mut output = vec![0u32; width * height];
    let result = match internalformat {
        // ETC2 RGB
        0x9274 => dec::decode_etc2_rgb(data, width, height, &mut output),
        // ETC2 RGBA1 (punchthrough alpha)
        0x9276 => dec::decode_etc2_rgba1(data, width, height, &mut output),
        // ETC2 RGBA8 (EAC)
        0x9278 => dec::decode_etc2_rgba8(data, width, height, &mut output),
        // ETC1
        0x8D64 => dec::decode_etc1(data, width, height, &mut output),
        // ASTC 4x4
        0x93B0 => dec::decode_astc_4_4(data, width, height, &mut output),
        // ASTC 6x6
        0x93B2 => dec::decode_astc_6_6(data, width, height, &mut output),
        // ASTC 8x8
        0x93B4 => dec::decode_astc_8_8(data, width, height, &mut output),
        // EAC R unsigned
        0x9270 => dec::decode_eacr(data, width, height, &mut output),
        // EAC RG unsigned
        0x9272 => dec::decode_eacrg(data, width, height, &mut output),
        _ => return None, // Not an Android format, let Mesa handle it
    };
    result.ok().map(|_| output)
}
```

### Phase 2: Integration with Binary Translator

Add graphics handling to the sober-core crate:

```toml
# Cargo.toml additions in sober-core
[dependencies]
egl-wrapper = { path = "../egl-wrapper" }
glesv2-wrapper = { path = "../glesv2-wrapper" }
texture2ddecoder = "0.1"
```

The binary translator needs to:
1. Pre-load `libEGL.so` and `libGLESv2.so` (our stubs or Mesa's real libs)
2. Map Android `ANativeWindow` → desktop native window (Wayland `wl_surface*` / X11 `Window`)
3. Route calls through the wrapper libraries

### Phase 3: Input Handling

Add a simple input crate:

```toml
# crates/input-wrapper/Cargo.toml
[dependencies]
sdl2 = "0.36"  # Or raw wayland-client / x11rb
```

Basic input flow:
```rust
// SDL2 event loop
loop {
    match event {
        Event::MouseButtonDown { x, y, .. } => {
            // Translate to Android ACTION_DOWN
            translate_input(ANDROID_ACTION_DOWN, x, y, 0);
        }
        Event::MouseButtonUp { .. } => {
            translate_input(ANDROID_ACTION_UP, x, y, 0);
        }
        Event::MouseMotion { x, y, .. } => {
            if mouse_down {
                translate_input(ANDROID_ACTION_MOVE, x, y, 0);
            }
        }
        Event::KeyDown { keycode, .. } => {
            // Translate keycodes to Android key events
            translate_key(keycode, ANDROID_KEY_ACTION_DOWN);
        }
        Event::Quit { .. } => break,
        _ => {}
    }
}
```

---

## 10. Summary of Recommendations

| Component | Approach | Priority |
|---|---|---|
| **EGL Implementation** | Use system Mesa `libEGL.so.1` (no custom EGL) | P0 — MVP |
| **GLES Implementation** | Use system Mesa `libGLESv2.so.2` (no custom GLES) | P0 — MVP |
| **GLES→Vulkan** | Set `MESA_LOADER_DRIVER_OVERRIDE=zink` to enable zink | P0 — MVP |
| **Window System** | Wayland primary, X11 fallback (Mesa handles both) | P0 — MVP |
| **Texture decompression** | Let Mesa handle ETC2 software decompression; add `texture2ddecoder` for ASTC if needed | P1 |
| **Input mapping** | SDL2 → translate mouse to Android touch events | P1 |
| **EGL stubs** | Create only if Mesa's soname doesn't match what Roblox expects | P2 |
| **ANGLE** | Build only if zink performance is unacceptable | P3 |
| **volk** | Not needed at application level (zink uses it internally) | P3 |
| **libhybris** | NOT recommended — it loads actual Android GPU drivers, not what we want | N/A |

### MVP Command to Launch Roblox

```bash
MESA_LOADER_DRIVER_OVERRIDE=zink \
EGL_PLATFORM=wayland \
open-sober run
```

This single environment variable tells Mesa to use the zink Gallium driver, which translates all OpenGL (including GLES) calls through Vulkan. Everything else (window creation, buffer swapping, shader compilation) is handled transparently by Mesa.