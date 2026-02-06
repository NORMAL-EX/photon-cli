# photon-cli 🔬

[![CI](https://github.com/NORMAL-EX/photon-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/NORMAL-EX/photon-cli/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)

> A physically-based Monte Carlo path tracer that renders 3D scenes directly in your terminal.

photon-cli solves the **rendering equation** using stochastic ray tracing, producing photorealistic images of 3D scenes — displayed right in your terminal using Unicode braille patterns, half-block characters, or ANSI true-color.

## ✨ Features

- **Physically-Based Rendering** — Full path tracing with the rendering equation: $L_o = L_e + \int f_r \cdot L_i \cdot \cos\theta \, d\omega$
- **Material System** — Lambertian diffuse, specular metals (Cook-Torrance approximation), dielectrics with Schlick-Fresnel and Snell's law, emissive area lights, and procedural checkerboard textures
- **BVH Acceleration** — $O(\log n)$ ray queries via bounding volume hierarchy with midpoint-split heuristic
- **Thin-Lens Camera** — Configurable field of view, focus distance, and aperture for depth-of-field bokeh
- **4 Output Modes** — Braille (2×4 subpixel), TrueColor, HalfBlock (2× vertical), and ASCII grayscale
- **4 Scene Presets** — Showcase, Cornell box, Minimal, and Stress test (500 spheres)
- **Cross-Platform** — Runs on Linux, macOS, and Windows
- **Zero Dependencies\*** — Only `clap` (CLI), `crossterm` (terminal), and `rand` (RNG)

## 📦 Installation

### From Source

```bash
git clone https://github.com/NORMAL-EX/photon-cli.git
cd photon-cli
cargo install --path .
```

### Pre-built Binaries

Download from the [Releases](https://github.com/NORMAL-EX/photon-cli/releases) page.

## 🚀 Usage

```bash
# Render the showcase scene (default)
photon-cli

# High-quality Cornell box render
photon-cli --scene cornell --spp 200 --bounces 20

# Quick preview with braille output (highest resolution)
photon-cli --scene minimal --mode braille --spp 8

# Large render
photon-cli --scene showcase -W 240 -H 120 --spp 100 --mode halfblock

# Stress test the BVH with 500 spheres
photon-cli --scene stress --spp 16
```

### CLI Options

| Flag | Description | Default |
|------|-------------|---------|
| `-s, --scene` | Scene preset (`showcase`, `cornell`, `minimal`, `stress`) | `showcase` |
| `-W, --width` | Output width in characters | `120` |
| `-H, --height` | Output height in characters | `60` |
| `--spp` | Samples per pixel (noise reduction) | `32` |
| `--bounces` | Maximum ray bounce depth | `12` |
| `-m, --mode` | Output mode (`braille`, `truecolor`, `halfblock`, `ascii`) | `halfblock` |
| `--no-gamma` | Disable sRGB gamma correction | `false` |

## 🎨 Output Modes

| Mode | Resolution | Description |
|------|-----------|-------------|
| `braille` | 2×4 subpixel | Unicode braille patterns (U+2800..U+28FF) with luminance thresholding and colored foreground |
| `truecolor` | 1:1 | Full-block `█` characters with 24-bit ANSI RGB foreground |
| `halfblock` | 1×2 | Upper-half-block `▀` with separate fg/bg colors — 2 vertical pixels per cell |
| `ascii` | 1:1 | Classic grayscale density ramp using BT.709 perceptual luminance |

## 🧬 Architecture

```
src/
├── main.rs        # CLI entry point (clap) and orchestration
├── math.rs        # Vec3, Ray, AABB — core linear algebra primitives
├── scene.rs       # Hittable trait, materials, geometry, BVH tree
├── camera.rs      # Thin-lens camera with depth-of-field
├── renderer.rs    # Path tracing integrator + terminal display engine
└── presets.rs     # Built-in scene descriptions
```

### Rendering Pipeline

```
Camera → Primary Ray → BVH Traversal → Hit Test → Material Scatter
                            ↑                           │
                            └───── Recursive Bounce ────┘
                                                        │
                                                        ↓
                              Framebuffer → Tone Map → Terminal Output
```

### Key Algorithms

- **Möller–Trumbore** triangle intersection (edge-vector + Cramer's rule)
- **Slab method** AABB intersection (branchless interval overlap)
- **Schlick approximation** for Fresnel reflectance in dielectrics
- **Cosine-weighted hemisphere** sampling for Lambertian importance sampling
- **Rejection sampling** for uniform sphere/disk point generation

## 📊 Performance

Approximate performance on AMD Ryzen 7 5800X (single-threaded):

| Scene | SPP | Resolution | Time | Mrays/s |
|-------|-----|-----------|------|---------|
| Minimal | 32 | 120×60 | ~2s | ~1.2 |
| Showcase | 32 | 120×60 | ~8s | ~0.9 |
| Cornell | 100 | 80×80 | ~12s | ~0.5 |
| Stress (500) | 16 | 120×60 | ~6s | ~0.6 |

## 📄 License

MIT — see [LICENSE](LICENSE).

## 🙏 Acknowledgments

- Inspired by [_Ray Tracing in One Weekend_](https://raytracing.github.io/) by Peter Shirley
- The [pbrt](https://pbrt.org/) reference for physically-based rendering theory
- Ferris the crab 🦀

---

Made with 🦀 by [NORMAL-EX](https://github.com/NORMAL-EX)
