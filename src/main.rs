//! # photon-cli 🔬
//!
//! A physically-based Monte Carlo path tracer that renders 3D scenes directly
//! in your terminal using Unicode braille patterns and ANSI true-color escape codes.
//!
//! ## Architecture
//!
//! The renderer implements a standard unidirectional path tracer with:
//! - **Geometric primitives**: Sphere, Plane, Triangle with BVH acceleration
//! - **Materials**: Lambertian, Metal, Dielectric (glass), Emissive, Checkerboard
//! - **Camera**: Thin-lens model with configurable DoF (depth of field)
//! - **Output modes**: Braille (2×4 subpixel), TrueColor, HalfBlock, ASCII
//!
//! ## Rendering equation
//!
//! The path tracer solves the rendering equation via Monte Carlo integration:
//!
//! ```text
//!   L_o(p, ω_o) = L_e(p, ω_o) + ∫_Ω f_r(p, ω_i, ω_o) · L_i(p, ω_i) · |cos θ_i| dω_i
//! ```
//!
//! Each material's `scatter` method importance-samples its BRDF lobe, and the
//! integrator recursively traces the scattered ray to evaluate `L_i`.

mod camera;
mod math;
mod presets;
mod renderer;
mod scene;

use clap::Parser;
use presets::ScenePreset;
use renderer::{display_framebuffer, OutputMode, PathTracer, RenderConfig};
use std::time::Instant;

/// photon-cli — render 3D scenes in your terminal
#[derive(Parser, Debug)]
#[command(
    name = "photon-cli",
    version,
    about = "A blazingly fast terminal ray tracer written in Rust 🦀",
    long_about = "Renders physically-based 3D scenes directly in your terminal using \
                  Monte Carlo path tracing. Supports multiple output modes from high-res \
                  braille patterns to simple ASCII art.",
    after_help = "EXAMPLES:\n  \
                  photon-cli --scene showcase --mode halfblock\n  \
                  photon-cli --scene cornell --spp 200 --bounces 20\n  \
                  photon-cli --scene minimal --width 240 --height 120 --mode braille\n  \
                  photon-cli --scene stress --spp 10"
)]
struct Cli {
    /// Scene preset to render
    #[arg(short, long, value_enum, default_value_t = ScenePreset::Showcase)]
    scene: ScenePreset,

    /// Output width in characters (actual pixel width depends on mode)
    #[arg(short = 'W', long, default_value_t = 120)]
    width: u32,

    /// Output height in characters
    #[arg(short = 'H', long, default_value_t = 60)]
    height: u32,

    /// Samples per pixel — higher values reduce noise at the cost of render time.
    /// 10–50 for previews, 200+ for high quality.
    #[arg(long, default_value_t = 32)]
    spp: u32,

    /// Maximum ray bounce depth. Higher values are needed for glass and
    /// complex interreflections. 8–16 is typically sufficient.
    #[arg(long, default_value_t = 12)]
    bounces: u32,

    /// Terminal output encoding mode
    #[arg(short, long, value_enum, default_value_t = CliOutputMode::Halfblock)]
    mode: CliOutputMode,

    /// Disable gamma correction (output linear radiance values directly)
    #[arg(long)]
    no_gamma: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum CliOutputMode {
    /// Unicode braille patterns — highest effective resolution (2×4 subpixel)
    Braille,
    /// Full-block characters with 24-bit true color
    Truecolor,
    /// Half-block characters (▀) — 2 vertical pixels per cell
    Halfblock,
    /// ASCII grayscale density ramp
    Ascii,
}

impl From<CliOutputMode> for OutputMode {
    fn from(m: CliOutputMode) -> Self {
        match m {
            CliOutputMode::Braille => OutputMode::Braille,
            CliOutputMode::Truecolor => OutputMode::TrueColor,
            CliOutputMode::Halfblock => OutputMode::HalfBlock,
            CliOutputMode::Ascii => OutputMode::Ascii,
        }
    }
}

fn print_header(scene_name: &str, config: &RenderConfig) {
    let mode_name = match config.output_mode {
        OutputMode::Braille => "Braille (2×4 subpixel)",
        OutputMode::TrueColor => "TrueColor (24-bit)",
        OutputMode::HalfBlock => "HalfBlock (2× vertical)",
        OutputMode::Ascii => "ASCII grayscale",
    };
    eprintln!();
    eprintln!("  ╔═══════════════════════════════════════════════╗");
    eprintln!("  ║  photon-cli 🔬  Terminal Path Tracer          ║");
    eprintln!("  ╚═══════════════════════════════════════════════╝");
    eprintln!();
    eprintln!("  Scene:    {scene_name}");
    eprintln!(
        "  Resolution: {}×{} ({mode_name})",
        config.width, config.height
    );
    eprintln!("  Samples:  {} spp", config.samples_per_pixel);
    eprintln!("  Bounces:  {}", config.max_bounces);
    eprintln!();
}

fn main() {
    let cli = Cli::parse();

    let scene_desc = cli.scene.build();
    let scene_name = scene_desc.name;

    let (world, camera, sky, mut config) = presets::build_world(scene_desc);

    // Override config with CLI arguments
    config.width = cli.width;
    config.height = cli.height;
    config.samples_per_pixel = cli.spp;
    config.max_bounces = cli.bounces;
    config.output_mode = cli.mode.into();
    config.gamma = !cli.no_gamma;

    print_header(scene_name, &config);

    let tracer = PathTracer {
        scene: &world,
        config: &config,
        camera: &camera,
        sky,
    };

    let t0 = Instant::now();
    let framebuffer = tracer.render();
    let elapsed = t0.elapsed();

    let total_rays = config.width as u64 * config.height as u64 * config.samples_per_pixel as u64;
    let mrays = total_rays as f64 / elapsed.as_secs_f64() / 1e6;

    eprintln!(
        "  Time: {:.2}s | {:.2}M rays | {:.2} Mrays/s",
        elapsed.as_secs_f64(),
        total_rays as f64 / 1e6,
        mrays
    );
    eprintln!();

    display_framebuffer(&framebuffer, config.output_mode);

    eprintln!();
    eprintln!("  Rendered with photon-cli v{}", env!("CARGO_PKG_VERSION"));
}
