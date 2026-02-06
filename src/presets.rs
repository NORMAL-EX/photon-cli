use crate::camera::{Camera, CameraConfig};
use crate::math::*;
use crate::renderer::{RenderConfig, SkyModel};
use crate::scene::*;
use rand::Rng;

/// A complete scene description bundling geometry, camera, lighting, and
/// render settings. Scene presets allow users to quickly render showcase
/// images without manual configuration.
pub struct SceneDescription {
    pub name: &'static str,
    pub objects: Vec<Box<dyn Hittable>>,
    pub camera_config: CameraConfig,
    pub sky: SkyModel,
}

/// Available built-in scene presets.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ScenePreset {
    /// The classic "Ray Tracing in One Weekend" spheres scene — a random
    /// arrangement of diffuse, metallic, and glass spheres on a checkerboard ground.
    Showcase,
    /// A Cornell box with quad walls, area light, and mixed materials.
    Cornell,
    /// A single reflective sphere on a ground plane — useful for benchmarking.
    Minimal,
    /// A gallery scene demonstrating all geometry types and materials.
    Gallery,
    /// A stress-test scene with many random objects to exercise BVH performance.
    Stress,
}

impl ScenePreset {
    pub fn build(self) -> SceneDescription {
        match self {
            ScenePreset::Showcase => build_showcase(),
            ScenePreset::Cornell => build_cornell(),
            ScenePreset::Minimal => build_minimal(),
            ScenePreset::Gallery => build_gallery(),
            ScenePreset::Stress => build_stress(),
        }
    }
}

fn build_showcase() -> SceneDescription {
    let mut objects: Vec<Box<dyn Hittable>> = Vec::new();
    let mut rng = rand::thread_rng();

    // Ground — checkerboard pattern
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, -1000.0, 0.0),
        1000.0,
        Checkerboard::new(
            Color::new(0.05, 0.05, 0.05),
            Color::new(0.95, 0.95, 0.95),
            10.0,
        ),
    )));

    // Three hero spheres
    // Glass sphere (center) with inner bubble for hollow effect
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, 1.0, 0.0),
        1.0,
        Dielectric::new(1.5),
    )));
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, 1.0, 0.0),
        -0.95,
        Dielectric::new(1.5),
    )));

    // Lambertian sphere (left)
    objects.push(Box::new(Sphere::new(
        Point3::new(-4.0, 1.0, 0.0),
        1.0,
        Lambertian::new(Color::new(0.7, 0.15, 0.15)),
    )));

    // Metal sphere (right)
    objects.push(Box::new(Sphere::new(
        Point3::new(4.0, 1.0, 0.0),
        1.0,
        Metal::new(Color::new(0.85, 0.85, 0.9), 0.0),
    )));

    // Random small spheres
    for a in -8..8 {
        for b in -8..8 {
            let center = Point3::new(
                a as f64 + 0.9 * rng.gen::<f64>(),
                0.2,
                b as f64 + 0.9 * rng.gen::<f64>(),
            );

            if (center - Point3::new(4.0, 0.2, 0.0)).length() < 0.9
                || (center - Point3::new(-4.0, 0.2, 0.0)).length() < 0.9
                || (center - Point3::new(0.0, 0.2, 0.0)).length() < 0.9
            {
                continue;
            }

            let choose_mat: f64 = rng.gen();
            let sphere: Box<dyn Hittable> = if choose_mat < 0.7 {
                let albedo = Color::new(
                    rng.gen::<f64>() * rng.gen::<f64>(),
                    rng.gen::<f64>() * rng.gen::<f64>(),
                    rng.gen::<f64>() * rng.gen::<f64>(),
                );
                Box::new(Sphere::new(center, 0.2, Lambertian::new(albedo)))
            } else if choose_mat < 0.9 {
                let albedo = Color::new(
                    rng.gen_range(0.5..1.0),
                    rng.gen_range(0.5..1.0),
                    rng.gen_range(0.5..1.0),
                );
                let fuzz = rng.gen_range(0.0..0.3);
                Box::new(Sphere::new(center, 0.2, Metal::new(albedo, fuzz)))
            } else {
                Box::new(Sphere::new(center, 0.2, Dielectric::new(1.5)))
            };
            objects.push(sphere);
        }
    }

    SceneDescription {
        name: "Showcase",
        objects,
        camera_config: CameraConfig {
            look_from: Point3::new(13.0, 2.0, 3.0),
            look_at: Point3::new(0.0, 0.5, 0.0),
            vup: Vec3::unit_y(),
            vfov_degrees: 20.0,
            aspect_ratio: 2.0,
            aperture: 0.1,
            focus_dist: 10.0,
        },
        sky: SkyModel::Gradient {
            horizon: Color::new(1.0, 1.0, 1.0),
            zenith: Color::new(0.5, 0.7, 1.0),
        },
    }
}

fn build_cornell() -> SceneDescription {
    let mut objects: Vec<Box<dyn Hittable>> = Vec::new();

    let white = Color::new(0.73, 0.73, 0.73);
    let red = Color::new(0.65, 0.05, 0.05);
    let green = Color::new(0.12, 0.45, 0.15);

    // Cornell box walls using Quad primitives for proper finite geometry
    // Floor
    objects.push(Box::new(Quad::new(
        Point3::new(-2.0, 0.0, -4.0),
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 4.0),
        Lambertian::new(white),
    )));

    // Ceiling
    objects.push(Box::new(Quad::new(
        Point3::new(-2.0, 4.0, -4.0),
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 4.0),
        Lambertian::new(white),
    )));

    // Back wall
    objects.push(Box::new(Quad::new(
        Point3::new(-2.0, 0.0, -4.0),
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(0.0, 4.0, 0.0),
        Lambertian::new(white),
    )));

    // Left wall (red)
    objects.push(Box::new(Quad::new(
        Point3::new(-2.0, 0.0, -4.0),
        Vec3::new(0.0, 0.0, 4.0),
        Vec3::new(0.0, 4.0, 0.0),
        Lambertian::new(red),
    )));

    // Right wall (green)
    objects.push(Box::new(Quad::new(
        Point3::new(2.0, 0.0, -4.0),
        Vec3::new(0.0, 0.0, 4.0),
        Vec3::new(0.0, 4.0, 0.0),
        Lambertian::new(green),
    )));

    // Area light on ceiling (small bright quad)
    objects.push(Box::new(Quad::new(
        Point3::new(-0.5, 3.99, -2.5),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Emissive::new(Color::new(1.0, 0.95, 0.85), 18.0),
    )));

    // Metal sphere (left)
    objects.push(Box::new(Sphere::new(
        Point3::new(-0.7, 0.6, -2.2),
        0.6,
        Metal::new(Color::new(0.9, 0.9, 0.95), 0.02),
    )));

    // Glass sphere (right)
    objects.push(Box::new(Sphere::new(
        Point3::new(0.7, 0.45, -1.5),
        0.45,
        Dielectric::new(1.5),
    )));

    SceneDescription {
        name: "Cornell Box",
        objects,
        camera_config: CameraConfig {
            look_from: Point3::new(0.0, 2.0, 3.5),
            look_at: Point3::new(0.0, 1.5, -2.0),
            vup: Vec3::unit_y(),
            vfov_degrees: 50.0,
            aspect_ratio: 1.0,
            aperture: 0.0,
            focus_dist: 5.0,
        },
        sky: SkyModel::Black,
    }
}

#[allow(clippy::vec_init_then_push)]
fn build_minimal() -> SceneDescription {
    let mut objects: Vec<Box<dyn Hittable>> = Vec::new();

    // Ground
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, -100.5, -1.0),
        100.0,
        Checkerboard::new(Color::new(0.1, 0.1, 0.1), Color::new(0.9, 0.9, 0.9), 15.0),
    )));

    // Chrome sphere
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, 0.5, -1.0),
        0.5,
        Metal::new(Color::new(0.95, 0.95, 0.97), 0.0),
    )));

    // Small colored spheres
    objects.push(Box::new(Sphere::new(
        Point3::new(-1.2, 0.25, -0.5),
        0.25,
        Lambertian::new(Color::new(0.9, 0.2, 0.1)),
    )));

    objects.push(Box::new(Sphere::new(
        Point3::new(1.0, 0.3, -0.8),
        0.3,
        Dielectric::new(1.5),
    )));

    SceneDescription {
        name: "Minimal",
        objects,
        camera_config: CameraConfig {
            look_from: Point3::new(0.0, 1.5, 2.0),
            look_at: Point3::new(0.0, 0.3, -1.0),
            vup: Vec3::unit_y(),
            vfov_degrees: 40.0,
            aspect_ratio: 2.0,
            aperture: 0.02,
            focus_dist: 3.0,
        },
        sky: SkyModel::Gradient {
            horizon: Color::new(1.0, 1.0, 1.0),
            zenith: Color::new(0.3, 0.5, 1.0),
        },
    }
}

/// Gallery scene — demonstrates every geometry type and material in one frame.
/// Features Quad backdrop, Disk platform, Gradient material, and mixed objects
/// arranged in an aesthetically pleasing composition.
#[allow(clippy::vec_init_then_push)]
fn build_gallery() -> SceneDescription {
    let mut objects: Vec<Box<dyn Hittable>> = Vec::new();

    // Ground — large checkerboard plane
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, -1000.0, 0.0),
        1000.0,
        Checkerboard::new(
            Color::new(0.08, 0.08, 0.12),
            Color::new(0.85, 0.85, 0.80),
            8.0,
        ),
    )));

    // Backdrop quad — a large matte panel behind the scene
    objects.push(Box::new(Quad::new(
        Point3::new(-6.0, 0.0, -5.0),
        Vec3::new(12.0, 0.0, 0.0),
        Vec3::new(0.0, 6.0, 0.0),
        Lambertian::new(Color::new(0.15, 0.15, 0.2)),
    )));

    // Disk pedestal — a reflective circular platform
    objects.push(Box::new(Disk::new(
        Point3::new(0.0, 0.01, -1.0),
        Vec3::unit_y(),
        2.5,
        Metal::new(Color::new(0.7, 0.7, 0.75), 0.15),
    )));

    // Center: large glass sphere with inner bubble
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, 1.0, -1.0),
        1.0,
        Dielectric::new(1.5),
    )));
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, 1.0, -1.0),
        -0.92,
        Dielectric::new(1.5),
    )));

    // Left: gradient material sphere (warm tones)
    objects.push(Box::new(Sphere::new(
        Point3::new(-2.8, 0.7, -0.5),
        0.7,
        GradientMaterial::new(
            Color::new(0.95, 0.3, 0.1),
            Color::new(0.95, 0.85, 0.2),
            Vec3::unit_y(),
        ),
    )));

    // Right: brushed metal sphere
    objects.push(Box::new(Sphere::new(
        Point3::new(2.8, 0.8, -0.8),
        0.8,
        Metal::new(Color::new(0.9, 0.75, 0.6), 0.08),
    )));

    // Small accent spheres
    objects.push(Box::new(Sphere::new(
        Point3::new(-1.2, 0.3, 0.8),
        0.3,
        Lambertian::new(Color::new(0.1, 0.4, 0.85)),
    )));

    objects.push(Box::new(Sphere::new(
        Point3::new(1.5, 0.25, 1.0),
        0.25,
        Metal::new(Color::new(0.95, 0.95, 0.95), 0.0),
    )));

    objects.push(Box::new(Sphere::new(
        Point3::new(0.8, 0.2, 0.5),
        0.2,
        Lambertian::new(Color::new(0.8, 0.15, 0.5)),
    )));

    // Floating emissive sphere (warm light source)
    objects.push(Box::new(Sphere::new(
        Point3::new(-1.0, 3.5, -2.0),
        0.3,
        Emissive::new(Color::new(1.0, 0.9, 0.7), 12.0),
    )));

    // Cool accent light
    objects.push(Box::new(Sphere::new(
        Point3::new(2.0, 2.5, 0.0),
        0.2,
        Emissive::new(Color::new(0.5, 0.7, 1.0), 10.0),
    )));

    SceneDescription {
        name: "Gallery",
        objects,
        camera_config: CameraConfig {
            look_from: Point3::new(0.0, 2.5, 6.0),
            look_at: Point3::new(0.0, 0.8, -1.0),
            vup: Vec3::unit_y(),
            vfov_degrees: 35.0,
            aspect_ratio: 16.0 / 9.0,
            aperture: 0.05,
            focus_dist: 7.0,
        },
        sky: SkyModel::Gradient {
            horizon: Color::new(0.15, 0.15, 0.2),
            zenith: Color::new(0.02, 0.02, 0.08),
        },
    }
}

fn build_stress() -> SceneDescription {
    let mut objects: Vec<Box<dyn Hittable>> = Vec::new();
    let mut rng = rand::thread_rng();

    // Ground
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, -1000.0, 0.0),
        1000.0,
        Lambertian::new(Color::new(0.5, 0.5, 0.5)),
    )));

    // 500 random spheres to stress-test BVH
    for _ in 0..500 {
        let center = Point3::new(
            rng.gen_range(-15.0..15.0),
            rng.gen_range(0.1..0.4),
            rng.gen_range(-15.0..15.0),
        );
        let radius = rng.gen_range(0.08..0.35);
        let albedo = Color::new(rng.gen(), rng.gen(), rng.gen());
        objects.push(Box::new(Sphere::new(
            center,
            radius,
            Lambertian::new(albedo),
        )));
    }

    SceneDescription {
        name: "Stress Test (500 spheres)",
        objects,
        camera_config: CameraConfig {
            look_from: Point3::new(10.0, 4.0, 10.0),
            look_at: Point3::zero(),
            vup: Vec3::unit_y(),
            vfov_degrees: 30.0,
            aspect_ratio: 2.0,
            aperture: 0.0,
            focus_dist: 14.0,
        },
        sky: SkyModel::Gradient {
            horizon: Color::new(1.0, 0.95, 0.88),
            zenith: Color::new(0.4, 0.6, 1.0),
        },
    }
}

/// Constructs the final renderable world from a scene description by
/// building a BVH over all objects for accelerated ray queries.
pub fn build_world(mut desc: SceneDescription) -> (BvhNode, Camera, SkyModel, RenderConfig) {
    let camera = Camera::new(&desc.camera_config);
    let aspect = desc.camera_config.aspect_ratio;

    let objects: Vec<Box<dyn Hittable>> = desc.objects.drain(..).collect();
    let bvh = BvhNode::build(objects);

    let config = RenderConfig {
        width: (80.0 * aspect) as u32,
        height: 80,
        ..Default::default()
    };

    (bvh, camera, desc.sky, config)
}
