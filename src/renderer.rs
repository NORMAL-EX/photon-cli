use crate::camera::Camera;
use crate::math::*;
use crate::scene::*;
use crossterm::style::{self, Stylize};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::io::{self, Write};

// ─── Render Configuration ───────────────────────────────────────────────────

pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    pub samples_per_pixel: u32,
    pub max_bounces: u32,
    pub output_mode: OutputMode,
    pub gamma: bool,
    pub tone_map: ToneMapOp,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 160,
            height: 80,
            samples_per_pixel: 50,
            max_bounces: 12,
            output_mode: OutputMode::TrueColor,
            gamma: true,
            tone_map: ToneMapOp::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMode {
    /// Unicode braille patterns (2x4 dots per cell) with ANSI true-color.
    Braille,
    /// ANSI 24-bit true-color using full-block characters.
    TrueColor,
    /// Half-block rendering with separate fg/bg colors — 2 vertical pixels per cell.
    HalfBlock,
    /// ASCII grayscale density ramp.
    Ascii,
}

// ─── Tone Mapping Operators ─────────────────────────────────────────────────

/// Tone mapping operators for HDR → LDR conversion. These compress the
/// high dynamic range radiance values into the displayable [0,1] range
/// while preserving perceptual contrast and color fidelity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToneMapOp {
    /// No tone mapping — clamp to [0,1] directly.
    None,
    /// Reinhard global operator (2002): L_d = L / (1 + L). Simple and robust,
    /// compresses highlights while preserving shadow detail. Works well
    /// for scenes with moderate dynamic range.
    Reinhard,
    /// ACES filmic tone mapping (Narkowicz 2015 approximation). The Academy Color
    /// Encoding System curve used in film production — produces rich,
    /// cinematic colors with a characteristic S-curve that lifts shadows
    /// and rolls off highlights smoothly.
    Aces,
}

impl ToneMapOp {
    /// Applies the tone mapping operator to a linear HDR color value.
    pub fn apply(self, color: Color) -> Color {
        match self {
            ToneMapOp::None => color,
            ToneMapOp::Reinhard => {
                // Reinhard global operator: x / (1 + x) per channel
                Color::new(
                    color.x / (1.0 + color.x),
                    color.y / (1.0 + color.y),
                    color.z / (1.0 + color.z),
                )
            }
            ToneMapOp::Aces => {
                // ACES filmic curve (Narkowicz 2015 approximation):
                //   f(x) = (x(2.51x + 0.03)) / (x(2.43x + 0.59) + 0.14)
                fn aces_channel(x: f64) -> f64 {
                    let a = 2.51;
                    let b = 0.03;
                    let c = 2.43;
                    let d = 0.59;
                    let e = 0.14;
                    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
                }
                Color::new(
                    aces_channel(color.x),
                    aces_channel(color.y),
                    aces_channel(color.z),
                )
            }
        }
    }
}

// ─── Framebuffer ────────────────────────────────────────────────────────────

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Color>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![Color::zero(); (width * height) as usize],
        }
    }

    #[inline]
    pub fn set(&mut self, x: u32, y: u32, color: Color) {
        self.pixels[(y * self.width + x) as usize] = color;
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> Color {
        self.pixels[(y * self.width + x) as usize]
    }

    /// Export the framebuffer as a PPM (Portable Pixmap) image file.
    /// PPM P6 binary format: RGB triplets, one byte per channel, no compression.
    /// This produces a lossless image that can be viewed with most image
    /// viewers or converted to PNG/JPEG with ImageMagick.
    pub fn write_ppm(&self, path: &str) -> io::Result<()> {
        let mut file = io::BufWriter::new(std::fs::File::create(path)?);
        write!(file, "P6\n{} {}\n255\n", self.width, self.height)?;
        for pixel in &self.pixels {
            let c = pixel.saturate();
            let r = (c.x * 255.999) as u8;
            let g = (c.y * 255.999) as u8;
            let b = (c.z * 255.999) as u8;
            file.write_all(&[r, g, b])?;
        }
        file.flush()?;
        Ok(())
    }
}

// ─── Render Statistics ──────────────────────────────────────────────────────

/// Aggregate statistics collected during the rendering pass for diagnostic output.
pub struct RenderStats {
    pub total_rays: u64,
    pub elapsed_secs: f64,
    pub width: u32,
    pub height: u32,
    pub spp: u32,
}

impl RenderStats {
    pub fn mrays_per_sec(&self) -> f64 {
        self.total_rays as f64 / self.elapsed_secs / 1e6
    }

    pub fn print_summary(&self) {
        let bar_width = 30;
        let fill = "━".repeat(bar_width);
        eprintln!("  {fill}");
        eprintln!("  Time:     {:.2}s", self.elapsed_secs);
        eprintln!("  Rays:     {:.2}M total", self.total_rays as f64 / 1e6);
        eprintln!("  Speed:    {:.2} Mrays/s", self.mrays_per_sec());
        eprintln!(
            "  Image:    {}×{} @ {} spp",
            self.width, self.height, self.spp
        );
        eprintln!("  {fill}");
    }
}

// ─── Progress Reporter ──────────────────────────────────────────────────────

/// A Unicode progress bar that renders to stderr with percentage, ETA, and a visual
/// bar using Unicode block characters for smooth sub-character progress.
struct ProgressBar {
    total: u32,
    done: u32,
    last_pct: u32,
    start: std::time::Instant,
}

impl ProgressBar {
    fn new(total: u32) -> Self {
        Self {
            total,
            done: 0,
            last_pct: 0,
            start: std::time::Instant::now(),
        }
    }

    fn tick(&mut self) {
        self.done += 1;
        let pct = self.done * 100 / self.total;
        if pct != self.last_pct {
            let elapsed = self.start.elapsed().as_secs_f64();
            let rate = self.done as f64 / elapsed;
            let remaining = (self.total - self.done) as f64 / rate;
            let bar_width = 24;
            let filled = (pct as usize * bar_width) / 100;
            let empty = bar_width - filled;
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
            eprint!("\r  Rendering: │{bar}│ {pct:3}%  ETA {:.0}s   ", remaining);
            self.last_pct = pct;
        }
    }

    fn finish(&self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        let bar = "█".repeat(24);
        eprintln!("\r  Rendering: │{bar}│ 100%  {:.2}s       ", elapsed);
    }
}

// ─── Path Tracer Integrator ─────────────────────────────────────────────────

/// Monte Carlo path tracing integrator solving the rendering equation:
///   L_o(p, ω_o) = L_e(p, ω_o) + ∫_Ω f_r(p, ω_i, ω_o) · L_i(p, ω_i) · |cos θ_i| dω_i
/// via importance-sampling the BRDF at each bounce.
pub struct PathTracer<'a> {
    pub scene: &'a dyn Hittable,
    pub config: &'a RenderConfig,
    pub camera: &'a Camera,
    pub sky: SkyModel,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SkyModel {
    Gradient { horizon: Color, zenith: Color },
    Solid(Color),
    Black,
}

impl SkyModel {
    pub fn sample(&self, ray: &Ray) -> Color {
        match self {
            SkyModel::Gradient { horizon, zenith } => {
                let unit_dir = ray.direction.normalized();
                let t = 0.5 * (unit_dir.y + 1.0);
                horizon.lerp(*zenith, t)
            }
            SkyModel::Solid(color) => *color,
            SkyModel::Black => Color::zero(),
        }
    }
}

impl<'a> PathTracer<'a> {
    /// Traces a single ray recursively through the scene, accumulating
    /// radiance from emissive surfaces and scattered light.
    fn trace_ray(&self, ray: &Ray, depth: u32, rng: &mut SmallRng) -> Color {
        if depth >= self.config.max_bounces {
            return Color::zero();
        }

        // t_min = 0.001 prevents shadow acne caused by floating-point self-intersection
        if let Some(hit) = self.scene.hit(ray, 0.001, f64::INFINITY) {
            let emitted = hit.material.emitted();

            if let Some((scattered, attenuation)) = hit.material.scatter(ray, &hit, rng) {
                let incoming = self.trace_ray(&scattered, depth + 1, rng);
                emitted + attenuation.hadamard(incoming)
            } else {
                emitted
            }
        } else {
            self.sky.sample(ray)
        }
    }

    /// Renders the full image into a framebuffer with stratified pixel sampling.
    /// Returns both the framebuffer and render statistics.
    pub fn render(&self) -> (Framebuffer, RenderStats) {
        let w = self.config.width;
        let h = self.config.height;
        let spp = self.config.samples_per_pixel;
        let mut fb = Framebuffer::new(w, h);
        let mut rng = SmallRng::from_entropy();

        let total = w * h;
        let mut progress = ProgressBar::new(total);
        let t0 = std::time::Instant::now();

        for y in (0..h).rev() {
            for x in 0..w {
                let mut pixel_color = Color::zero();
                for _ in 0..spp {
                    let u = (x as f64 + rng.gen::<f64>()) / (w - 1) as f64;
                    let v = (y as f64 + rng.gen::<f64>()) / (h - 1) as f64;
                    let ray = self.camera.get_ray(u, v, &mut rng);
                    pixel_color += self.trace_ray(&ray, 0, &mut rng);
                }
                pixel_color /= spp as f64;

                // Apply tone mapping in linear space before gamma correction
                pixel_color = self.config.tone_map.apply(pixel_color);

                if self.config.gamma {
                    pixel_color = pixel_color.gamma_correct();
                }

                fb.set(x, h - 1 - y, pixel_color);
                progress.tick();
            }
        }
        progress.finish();

        let elapsed = t0.elapsed();
        let total_rays = w as u64 * h as u64 * spp as u64;

        let stats = RenderStats {
            total_rays,
            elapsed_secs: elapsed.as_secs_f64(),
            width: w,
            height: h,
            spp,
        };

        (fb, stats)
    }
}

// ─── Terminal Display Engine ────────────────────────────────────────────────

pub fn display_framebuffer(fb: &Framebuffer, mode: OutputMode) {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    match mode {
        OutputMode::TrueColor => display_truecolor(&mut out, fb),
        OutputMode::HalfBlock => display_halfblock(&mut out, fb),
        OutputMode::Ascii => display_ascii(&mut out, fb),
        OutputMode::Braille => display_braille(&mut out, fb),
    }
    let _ = out.flush();
}

fn display_truecolor(out: &mut impl Write, fb: &Framebuffer) {
    for y in 0..fb.height {
        for x in 0..fb.width {
            let (r, g, b) = fb.get(x, y).to_rgb8();
            let _ = write!(out, "{}", "█".with(style::Color::Rgb { r, g, b }));
        }
        let _ = writeln!(out);
    }
}

fn display_halfblock(out: &mut impl Write, fb: &Framebuffer) {
    let rows = fb.height / 2;
    for row in 0..rows {
        for x in 0..fb.width {
            let (tr, tg, tb) = fb.get(x, row * 2).to_rgb8();
            let (br, bg, bb) = fb.get(x, row * 2 + 1).to_rgb8();
            let _ = write!(
                out,
                "{}",
                "▀"
                    .with(style::Color::Rgb {
                        r: tr,
                        g: tg,
                        b: tb
                    })
                    .on(style::Color::Rgb {
                        r: br,
                        g: bg,
                        b: bb
                    })
            );
        }
        let _ = writeln!(out);
    }
}

fn display_ascii(out: &mut impl Write, fb: &Framebuffer) {
    const RAMP: &[u8] = b" .:-=+*#%@";
    for y in 0..fb.height {
        for x in 0..fb.width {
            let c = fb.get(x, y);
            let lum = 0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
            let idx = (lum.clamp(0.0, 0.999) * RAMP.len() as f64) as usize;
            let _ = write!(out, "{}", RAMP[idx] as char);
        }
        let _ = writeln!(out);
    }
}

/// Braille pattern rendering — each Unicode braille char (U+2800..U+28FF) encodes
/// a 2x4 dot matrix, achieving 2× horizontal and 4× vertical subpixel resolution.
///
/// Dot-to-bit mapping (Unicode standard):
///   ┌───┐
///   │ 0 3 │    Bits 0-5 → dots 0-5
///   │ 1 4 │    Bit 6   → dot 6
///   │ 2 5 │    Bit 7   → dot 7
///   │ 6 7 │
///   └───┘
fn display_braille(out: &mut impl Write, fb: &Framebuffer) {
    let cell_w = 2u32;
    let cell_h = 4u32;
    let cols = fb.width / cell_w;
    let rows = fb.height / cell_h;

    for row in 0..rows {
        for col in 0..cols {
            let bx = col * cell_w;
            let by = row * cell_h;

            let mut pattern: u8 = 0;
            let mut avg_color = Color::zero();
            let mut lit_count = 0u32;

            let offsets: [(u32, u32, u8); 8] = [
                (0, 0, 0),
                (0, 1, 1),
                (0, 2, 2),
                (1, 0, 3),
                (1, 1, 4),
                (1, 2, 5),
                (0, 3, 6),
                (1, 3, 7),
            ];

            for &(dx, dy, bit) in &offsets {
                let px = bx + dx;
                let py = by + dy;
                if px < fb.width && py < fb.height {
                    let c = fb.get(px, py);
                    let lum = 0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
                    if lum > 0.15 {
                        pattern |= 1 << bit;
                        avg_color += c;
                        lit_count += 1;
                    }
                }
            }

            if lit_count > 0 {
                avg_color /= lit_count as f64;
            }

            let braille_char = char::from_u32(0x2800 + pattern as u32).unwrap_or(' ');
            let (r, g, b) = avg_color.to_rgb8();
            let _ = write!(
                out,
                "{}",
                braille_char.to_string().with(style::Color::Rgb { r, g, b })
            );
        }
        let _ = writeln!(out);
    }
}
