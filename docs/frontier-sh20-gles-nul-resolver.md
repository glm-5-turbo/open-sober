# SH20 (2026-09-12) — resolve_gles_int / resolve_egl accept trailing-NUL names; all 8 engine GLES dispatch slots now resolve through the bridge. Workspace 471/0.

## One-line result
The engine's OWN frame-fn 0x105b32c00 now dispatches its ENTIRE clear/draw path
through the host-thunk GLES bridge: all 8 dispatch slots (BSS 0x106d3b2f0..328,
int + float) resolve to bridge slots, not just the 2 float slots SH19 could seed.
Return Ok, post-frame swap Ok(0x1), stable exit 124, no crash.

## Root cause (latent, SH20)
`resolve_gles_int` built its CString cache key from the RAW `name` argument. When
a caller passed a NUL-terminated name — which elfjit's `--renderframe-seedgles`
does via `format!("{name}\0")`, and which `w_eglGetProcAddress` does for a guest
C-string — `CString::new` rejected the interior NUL and the resolver returned
`None`, even for perfectly-whitelisted int-ABI names (glClear/glViewport/
glColorMask/glDepthMask/glStencilMask/glClearStencil) that Mesa exports.
`resolve_gles_mixed` strips the NUL via `name_str` first (so it worked — that's
why slots 0/2 float slots seeded in SH19); the int resolver did not. The same
latent bug existed in `resolve_egl`.

Fix: build the CString from the NUL-stripped `ns` in both. Regression test
`resolve_gles_int_accepts_trailing_nul_like_mixed` pins all 8 names resolve with
a trailing NUL.

## Real-binary verification (this is the point)
Run-log runs/sh20-seedgles-all-slots-ok.txt (command there). Slot dump BEFORE
(SH19) vs AFTER (SH20):

  slot  name            SH19                          SH20
  0    glClearColor    <- bridge 0x7f0000018050      <- bridge 0x7f0000018050  (unchanged)
  1    glClear         NOT resolvable                <- bridge 0x7f00000029c0   [NEW]
  2    glClearDepthf   <- bridge 0x7f0000018058      <- bridge 0x7f0000018058  (unchanged)
  3    glClearStencil  NOT resolvable                <- bridge 0x7f00000029b8  [NEW]
  4    glColorMask     NOT resolvable                <- bridge 0x7f0000002978  [NEW]
  5    glDepthMask     NOT resolvable                <- bridge 0x7f0000002998  [NEW]
  6    glStencilMask   NOT resolvable                <- bridge 0x7f00000029b0  [NEW]
  7    glViewport      NOT resolvable                <- bridge 0x7f0000002918  [NEW]

The 6 int slots were raw-Mesa-or-garbage before; now they route through our
bridge (int-ABI slots, float/texture interception preserved).

## Un-skipped graphics gate
`resolve_gles_mixed_float_and_stack_abi_execute_real_mesa` silently SKIPPED its
whole EGL/GLES body for its entire life: every NUL-terminated `resolve_egl`
call returned None -> `else return`. SH20 makes it actually run a real
surfaceless EGL -> ES3 -> GLES chain through the JIT bridges and pass. That
exposed and fixed 3 latent harness bugs:
  1. eglChooseConfig / eglCreateContext attrib arrays must be i32 (EGLint*),
     not u64, else Mesa reads misaligned attrib pairs -> 0 configs.
  2. surfaceless EGL needs a bound pbuffer surface (not EGL_NO_SURFACE) for a
     queryable draw buffer.
  3. Mesa surfaceless llvmpipe reports GL_INVALID_ENUM for
     glGetFloatv(GL_COLOR_CLEAR_VALUE), so the state-query round-trip is
     replaced with glClear + glReadPixels real-pixel verification (float-bridge
     clear color renders actual [132,65,189,255] pixels).

## Next lever (unchanged frontier)
The frame-fn returns Ok through the bridge; the remaining wall is the coherent
renderer/view C++ object reverse for the full draw path, plus the ALooper /
lifecycle producer (START cmd) that would drive the loop from the engine's main
thread (SH14-capped). Baselines unchanged: --jni exit 0; stable idle exit 124.
Workspace 471/0.