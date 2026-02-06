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
    /// A minimal Cornell box-inspired scene demonstrating emissive lighting,
    /// diffuse interreflection, and color bleeding.
    Cornell,
    /// A single reflective sphere on a ground plane — useful for benchmarking
    /// and verifying reflection/refraction correctness.
    Minimal,
    /// A stress-test scene with many random objects to exercise BVH performance.
    Stress,
}

impl ScenePreset {
    pub fn build(self) -> SceneDescription {
        match self {
            ScenePreset::Showcase => build_showcase(),
            ScenePreset::Cornell => build_cornell(),
            ScenePreset::Minimal => build_minimal(),
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
    // Glass sphere (center)
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, 1.0, 0.0),
        1.0,
        Dielectric::new(1.5),
    )));

    // Inner bubble for hollow glass effect
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

    // Random small spheres scattered around the scene
    for a in -8..8 {
        for b in -8..8 {
            let center = Point3::new(
                a as f64 + 0.9 * rng.gen::<f64>(),
                0.2,
                b as f64 + 0.9 * rng.gen::<f64>(),
            );

            // Skip positions too close to the hero spheres
            if (center - Point3::new(4.0, 0.2, 0.0)).length() < 0.9
                || (center - Point3::new(-4.0, 0.2, 0.0)).length() < 0.9
                || (center - Point3::new(0.0, 0.2, 0.0)).length() < 0.9
            {
                continue;
            }

            let choose_mat: f64 = rng.gen();
            let sphere: Box<dyn Hittable> = if choose_mat < 0.7 {
                // Diffuse
                let albedo = Color::new(
                    rng.gen::<f64>() * rng.gen::<f64>(),
                    rng.gen::<f64>() * rng.gen::<f64>(),
                    rng.gen::<f64>() * rng.gen::<f64>(),
                );
                Box::new(Sphere::new(center, 0.2, Lambertian::new(albedo)))
            } else if choose_mat < 0.9 {
                // Metal
                let albedo = Color::new(
                    rng.gen_range(0.5..1.0),
                    rng.gen_range(0.5..1.0),
                    rng.gen_range(0.5..1.0),
                );
                let fuzz = rng.gen_range(0.0..0.3);
                Box::new(Sphere::new(center, 0.2, Metal::new(albedo, fuzz)))
            } else {
                // Glass
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

    // Floor
    objects.push(Box::new(Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Vec3::unit_y(),
        Lambertian::new(Color::new(0.73, 0.73, 0.73)),
    )));

    // Back wall
    objects.push(Box::new(Plane::new(
        Point3::new(0.0, 0.0, -3.0),
        Vec3::unit_z(),
        Lambertian::new(Color::new(0.73, 0.73, 0.73)),
    )));

    // Left wall (red)
    objects.push(Box::new(Sphere::new(
        Point3::new(-1002.0, 0.0, 0.0),
        1000.0,
        Lambertian::new(Color::new(0.65, 0.05, 0.05)),
    )));

    // Right wall (green)
    objects.push(Box::new(Sphere::new(
        Point3::new(1002.0, 0.0, 0.0),
        1000.0,
        Lambertian::new(Color::new(0.12, 0.45, 0.15)),
    )));

    // Two spheres inside
    objects.push(Box::new(Sphere::new(
        Point3::new(-0.6, 0.5, -1.5),
        0.5,
        Metal::new(Color::new(0.9, 0.9, 0.95), 0.02),
    )));

    objects.push(Box::new(Sphere::new(
        Point3::new(0.6, 0.35, -1.0),
        0.35,
        Dielectric::new(1.5),
    )));

    // Light source (small bright sphere at top)
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, 3.0, -1.5),
        0.5,
        Emissive::new(Color::new(1.0, 0.95, 0.85), 15.0),
    )));

    SceneDescription {
        name: "Cornell Box",
        objects,
        camera_config: CameraConfig {
            look_from: Point3::new(0.0, 1.0, 3.5),
            look_at: Point3::new(0.0, 0.5, -1.0),
            vup: Vec3::unit_y(),
            vfov_degrees: 45.0,
            aspect_ratio: 1.0,
            aperture: 0.0,
            focus_dist: 4.0,
        },
        sky: SkyModel::Black,
    }
}

fn build_minimal() -> SceneDescription {
    let mut objects: Vec<Box<dyn Hittable>> = Vec::new();

    // Ground
    objects.push(Box::new(Sphere::new(
        Point3::new(0.0, -100.5, -1.0),
        100.0,
        Checkerboard::new(
            Color::new(0.1, 0.1, 0.1),
            Color::new(0.9, 0.9, 0.9),
            15.0,
        ),
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
        objects.push(Box::new(Sphere::new(center, radius, Lambertian::new(albedo))));
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

    // Separate BVH-compatible and infinite objects
    let objects: Vec<Box<dyn Hittable>> = desc.objects.drain(..).collect();
    let bvh = BvhNode::build(objects);

    let config = RenderConfig {
        width: (80.0 * aspect) as u32,
        height: 80,
        ..Default::default()
    };

    (bvh, camera, desc.sky, config)
}
