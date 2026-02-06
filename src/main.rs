//! # photon-cli 🔬
//!
//! A physically-based Monte Carlo path tracer that renders 3D scenes directly
//! in your terminal using Unicode braille patterns and ANSI true-color escape codes.
//!
//! ## Architecture
//!
//! The renderer implements a standard unidirectional path tracer with:
//! - **Geometric primitives**: Sphere, Plane, Triangle, Quad, Disk with BVH acceleration
//! - **Materials**: Lambertian, Metal, Dielectric (glass), Emissive, Checkerboard, Gradient
//! - **Camera**: Thin-lens model with configurable DoF (depth of field)
//! - **Output modes**: Braille (2×4 subpixel), TrueColor, HalfBlock, ASCII
//! - **Tone mapping**: None, Reinhard, ACES filmic
//! - **Export**: PPM image file output
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
use renderer::{display_framebuffer, OutputMode, PathTracer, RenderConfig, ToneMapOp};

/// photon-cli — render 3D scenes in your terminal
#[derive(Parser, Debug)]
#[command(
    name = "photon-cli",
    version,
    about = "A blazingly fast terminal ray tracer written in Rust 🦀",
    long_about = "Renders physically-based 3D scenes directly in your terminal using \
                  Monte Carlo path tracing. Supports multiple output modes from high-res \
                  braille patterns to simple ASCII art, with ACES/Reinhard tone mapping \
                  and PPM image export.",
    after_help = "EXAMPLES:\n  \
                  photon-cli --scene showcase --mode halfblock\n  \
                  photon-cli --scene cornell --spp 200 --bounces 20 --tonemap aces\n  \
                  photon-cli --scene minimal --width 240 --height 120 --mode braille\n  \
                  photon-cli --scene gallery --spp 64 --tonemap reinhard\n  \
                  photon-cli --scene stress --spp 10 --output render.ppm"
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

    /// Tone mapping operator for HDR → LDR conversion
    #[arg(short, long, value_enum, default_value_t = CliToneMap::None)]
    tonemap: CliToneMap,

    /// Disable gamma correction (output linear radiance values directly)
    #[arg(long)]
    no_gamma: bool,

    /// Save rendered image to a PPM file (in addition to terminal display)
    #[arg(short, long)]
    output: Option<String>,

    /// Suppress terminal display (useful with --output for headless rendering)
    #[arg(long)]
    quiet: bool,
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

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum CliToneMap {
    /// No tone mapping — clamp to [0,1] directly
    None,
    /// Reinhard global operator: L/(1+L)
    Reinhard,
    /// ACES filmic curve (cinematic look)
    Aces,
}

impl From<CliToneMap> for ToneMapOp {
    fn from(t: CliToneMap) -> Self {
        match t {
            CliToneMap::None => ToneMapOp::None,
            CliToneMap::Reinhard => ToneMapOp::Reinhard,
            CliToneMap::Aces => ToneMapOp::Aces,
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
    let tonemap_name = match config.tone_map {
        ToneMapOp::None => "None (clamp)",
        ToneMapOp::Reinhard => "Reinhard",
        ToneMapOp::Aces => "ACES Filmic",
    };
    eprintln!();
    eprintln!("  ╔═══════════════════════════════════════════════╗");
    eprintln!("  ║  photon-cli 🔬  Terminal Path Tracer          ║");
    eprintln!("  ╚═══════════════════════════════════════════════╝");
    eprintln!();
    eprintln!("  Scene:      {scene_name}");
    eprintln!(
        "  Resolution: {}×{} ({mode_name})",
        config.width, config.height
    );
    eprintln!("  Samples:    {} spp", config.samples_per_pixel);
    eprintln!("  Bounces:    {}", config.max_bounces);
    eprintln!("  Tone map:   {tonemap_name}");
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
    config.tone_map = cli.tonemap.into();
    config.gamma = !cli.no_gamma;

    print_header(scene_name, &config);

    // Print BVH diagnostics
    eprintln!(
        "  BVH:        {} objects, depth {}",
        world.leaf_count(),
        world.depth()
    );
    eprintln!();

    let tracer = PathTracer {
        scene: &world,
        config: &config,
        camera: &camera,
        sky,
    };

    let (framebuffer, stats) = tracer.render();
    eprintln!();
    stats.print_summary();
    eprintln!();

    // Terminal display
    if !cli.quiet {
        display_framebuffer(&framebuffer, config.output_mode);
    }

    // PPM export
    if let Some(ref path) = cli.output {
        match framebuffer.write_ppm(path) {
            Ok(()) => eprintln!("  Saved: {path}"),
            Err(e) => eprintln!("  Error saving {path}: {e}"),
        }
    }

    eprintln!();
    eprintln!("  Rendered with photon-cli v{}", env!("CARGO_PKG_VERSION"));
}
