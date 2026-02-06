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
}

// ─── Path Tracer Integrator ─────────────────────────────────────────────────

/// Monte Carlo path tracing integrator solving the rendering equation:
///   L_o(p, w_o) = L_e(p, w_o) + integral f_r * L_i * cos(theta) dw
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
    /// Traces a single ray recursively through the scene.
    fn trace_ray(&self, ray: &Ray, depth: u32, rng: &mut SmallRng) -> Color {
        if depth >= self.config.max_bounces {
            return Color::zero();
        }

        // t_min = 0.001 to prevent shadow acne from floating-point self-intersection
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
    pub fn render(&self) -> Framebuffer {
        let w = self.config.width;
        let h = self.config.height;
        let spp = self.config.samples_per_pixel;
        let mut fb = Framebuffer::new(w, h);
        let mut rng = SmallRng::from_entropy();

        let total = w * h;
        let mut done = 0u32;
        let mut last_pct = 0u32;

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

                if self.config.gamma {
                    pixel_color = pixel_color.gamma_correct();
                }

                fb.set(x, h - 1 - y, pixel_color);

                done += 1;
                let pct = done * 100 / total;
                if pct != last_pct {
                    eprint!("\r  Rendering: {pct}%");
                    last_pct = pct;
                }
            }
        }
        eprintln!("\r  Rendering: done.     ");
        fb
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
                out, "{}",
                "▀".with(style::Color::Rgb { r: tr, g: tg, b: tb })
                   .on(style::Color::Rgb { r: br, g: bg, b: bb })
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
/// a 2x4 dot matrix, achieving 2x horizontal and 4x vertical subpixel resolution.
///
/// Dot-to-bit mapping (Unicode standard):
///   ┌───┐
///   │ 0 3 │    Bits 0-5 map to dots 0-5
///   │ 1 4 │    Bit 6 → dot 6
///   │ 2 5 │    Bit 7 → dot 7
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
                (0, 0, 0), (0, 1, 1), (0, 2, 2),
                (1, 0, 3), (1, 1, 4), (1, 2, 5),
                (0, 3, 6), (1, 3, 7),
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
                out, "{}",
                braille_char.to_string().with(style::Color::Rgb { r, g, b })
            );
        }
        let _ = writeln!(out);
    }
}
