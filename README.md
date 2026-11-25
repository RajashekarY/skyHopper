# skyHopper
A Super Mario like Game build using Rust, Bevy

A small original 3D platformer in Rust + Bevy 0.20 (edition 2024). Pip runs, double-jumps, stomps a Walker, grabs 12 coins and reaches the golden goal.

## Run
```bash
cargo run --release
```
Linux needs: `libudev-dev libwayland-dev libxkbcommon-dev pkg-config` (Debian/Ubuntu names). Windows and macOS need nothing extra.

On the first launch you pick a run mode (Low / Medium / High). Tick **Don't ask me again** to skip the dialog next time. Change it any time with **Esc → Preferences**.

| Mode | What it turns on |
|---|---|
| Low | no shadow maps (blob shadow only), no AA, no trees, short view distance |
| Medium | 1-cascade shadows (1024), FXAA, half the trees |
| High | 3-cascade shadows (2048), MSAA 4x, HDR bloom, all trees, far view |

VSync is off by default (uncapped FPS). F3 toggles the FPS counter.

## Controls
WASD / left stick to move · Space / A to jump (press again in the air to double jump) · mouse / right stick to move the camera · hold right mouse / LT to look through Pip's eyes · Esc / Start to pause.

## No working GPU driver?
The game tries Vulkan, DX12, Metal and OpenGL automatically. To force a backend use `WGPU_BACKEND=gl` (or `vulkan`, `dx12`, `metal`). As a last resort, `WGPU_FORCE_FALLBACK_ADAPTER=1` uses the CPU software renderer (slow, but it runs).

## Tests
```bash
cargo test
```
