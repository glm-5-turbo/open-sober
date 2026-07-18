# Open Sober

Open-source reimplementation of the VinegarHQ Sober Roblox Linux runtime.

Sober runs the **Android ARM64 Roblox APK natively on x86-64 Linux** via
binary translation, bypassing Hyperion (Roblox's anticheat, not present in
Android builds). The original Sober by VinegarHQ is closed-source — this is
a clean-room reimplementation.

## Architecture

| Crate | Status | Description |
|---|---|---|
| **libbadcpu** | ✅ Phase 1 | x86-64 CPU feature emulator (SIGILL handler) |
| **libloader** | ⬜ Planned | Process spawner/sandboxer |
| **sober-core** | ⬜ Planned | Android binary translation engine |
| **sober-services** | ⬜ Planned | Auth/launcher GUI |

## How It Works

1. **sober-services** opens a WebKit webview for Roblox OAuth login
2. Auth tokens passed to **sober-core** via IPC
3. **sober-core** downloads the Roblox Android APK, initializes binary translator
4. **libloader** forks, chroots, mmaps the translated binary
5. **libbadcpu** handles missing CPU instructions via SIGILL
6. Roblox runs on Linux with translated syscalls + GLES→Vulkan

## libbadcpu — Emulated Instructions

POPCNT, MOVBE, LZCNT, TZCNT, ANDN, BLSI, BLSMSK, BLSR

## Build

```bash
cargo build
cargo test
```

## License

MIT
