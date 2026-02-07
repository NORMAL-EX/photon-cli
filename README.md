# photon-cli 🔬

[![CI](https://github.com/NORMAL-EX/photon-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/NORMAL-EX/photon-cli/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)

> A physically-based Monte Carlo path tracer that renders 3D scenes directly in your terminal.

photon-cli solves the **rendering equation** using stochastic ray tracing, producing photorealistic images of 3D scenes — displayed right in your terminal using Unicode braille patterns, half-block characters, or ANSI true-color.

## ✨ Features

- **Physically-Based Rendering** — Full path tracing solving the rendering equation: $L_o = L_e + \int_{\Omega} f_r \cdot L_i \cdot \cos\theta \, d\omega$
- **Material System** — Lambertian diffuse, specular metals (Cook-Torrance), dielectrics with Schlick-Fresnel, emissive area lights, procedural checkerboard, and normal-driven gradients
- **Geometry Primitives** — Sphere, Plane, Triangle (Möller–Trumbore), Quad (parametric rectangle), Disk
- **BVH Acceleration** — $O(\log n)$ ray queries via bounding volume hierarchy with midpoint-split heuristic
- **Thin-Lens Camera** — Configurable FOV, focus distance, and aperture for depth-of-field bokeh
- **Tone Mapping** — None (clamp), Reinhard global operator, and ACES filmic curve
- **4 Output Modes** — Braille (2×4 subpixel), TrueColor, HalfBlock (2× vertical), ASCII grayscale
- **5 Scene Presets** — Showcase, Cornell box, Minimal, Gallery, Stress test (500 spheres)
- **PPM Export** — Save renders to lossless PPM image files
- **Cross-Platform** — Runs on Linux, macOS, and Windows

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

# High-quality Cornell box with ACES tone mapping
photon-cli --scene cornell --spp 200 --bounces 20 --tonemap aces

# Quick preview with braille output (highest resolution)
photon-cli --scene minimal --mode braille --spp 8

# Gallery scene with all geometry types and materials
photon-cli --scene gallery --spp 64 --tonemap reinhard

# Large render saved to PPM file
photon-cli --scene showcase -W 240 -H 120 --spp 100 --output render.ppm

# Headless rendering (no terminal display)
photon-cli --scene cornell --spp 500 --output hq.ppm --quiet
```

### CLI Options

| Flag | Description | Default |
|------|-------------|---------|-
| `-s, --scene` | Scene preset (`showcase`, `cornell`, `minimal`, `gallery`, `stress`) | `showcase` |
| `-W, --width` | Output width in characters | `120` |
| `-H, --height` | Output height in characters | `60` |
| `--spp` | Samples per pixel (noise reduction) | `32` |
| `--bounces` | Maximum ray bounce depth | `12` |
| `-m, --mode` | Output mode (`braille`, `truecolor`, `halfblock`, `ascii`) | `halfblock` |
| `-t, --tonemap` | Tone mapping (`none`, `reinhard`, `aces`) | `none` |
| `-o, --output` | Save render to PPM file | — |
| `--quiet` | Suppress terminal display | `false` |
| `--no-gamma` | Disable sRGB gamma correction | `false` |

## 🎨 Output Modes

| Mode | Resolution | Description |
|------|-----------|-------------|
| `braille` | 2×4 subpixel | Unicode braille patterns (U+2800..U+28FF) with luminance thresholding |
| `truecolor` | 1:1 | Full-block `█` characters with 24-bit ANSI RGB |
| `halfblock` | 1×2 | Upper-half-block `▀` with separate fg/bg colors |
| `ascii` | 1:1 | Classic grayscale density ramp using BT.709 luminance |

## 🎬 Tone Mapping

| Operator | Formula | Best For |
|----------|---------|----------|
| `none` | Clamp to [0,1] | Outdoor scenes with moderate dynamic range |
| `reinhard` | $L_d = L / (1 + L)$ | General purpose, preserves shadow detail |
| `aces` | ACES filmic S-curve | Cinematic look, rich colors, smooth highlight rolloff |

## 🧬 Architecture

```
src/
├── main.rs        # CLI entry point (clap) and orchestration
├── math.rs        # Vec3, Ray, AABB — core linear algebra primitives
├── scene.rs       # Hittable trait, materials, geometry, BVH tree
├── camera.rs      # Thin-lens camera with depth-of-field
├── renderer.rs    # Path tracing integrator, tone mapping, display engine
└── presets.rs     # Built-in scene descriptions
```

### Rendering Pipeline

```
Camera → Primary Ray → BVH Traversal → Hit Test → Material Scatter
                            ↑                           │
                            └───── Recursive Bounce ────┘
                                                        │
                                                        ↓
                     Framebuffer → Tone Map → Gamma → Terminal / PPM
```

### Key Algorithms

- **Möller–Trumbore** triangle intersection (edge-vector + Cramer's rule)
- **Slab method** AABB intersection (branchless interval overlap)
- **Schlick approximation** for Fresnel reflectance in dielectrics
- **Cosine-weighted hemisphere** sampling for Lambertian importance sampling
- **Parametric quad** intersection with cross-product coordinate extraction
- **ACES filmic** tone mapping (Narkowicz 2015 polynomial fit)
- **Reinhard** global tone mapping operator

## 📄 License

MIT — see [LICENSE](LICENSE).

## 🙏 Acknowledgments

- Inspired by [_Ray Tracing in One Weekend_](https://raytracing.github.io/) by Peter Shirley
- The [pbrt](https://pbrt.org/) reference for physically-based rendering theory
- ACES tone mapping from [Narkowicz 2015](https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/)
- Ferris the crab 🦀

---

Made with 🦀 by [NORMAL-EX](https://github.com/NORMAL-EX)
