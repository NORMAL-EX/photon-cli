use crate::math::*;

/// A thin-lens camera model with configurable field of view, aspect ratio,
/// focus distance, and aperture size. The camera constructs an orthonormal
/// basis (u, v, w) from the look-at parameters, then generates primary rays
/// by mapping pixel coordinates to points on the virtual film plane.
///
/// Depth of field is simulated by jittering the ray origin across a disk
/// of radius `aperture/2` centered at the camera position, while keeping
/// the focal point fixed. This produces the characteristic bokeh blur for
/// objects not at the focus distance.
pub struct Camera {
    origin: Point3,
    lower_left: Point3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    lens_radius: f64,
}

/// Configuration builder for the camera, following the builder pattern
/// to allow incremental, readable camera setup.
/// Configuration for the thin-lens camera model with depth-of-field.
pub struct CameraConfig {
    pub look_from: Point3,
    pub look_at: Point3,
    pub vup: Vec3,
    pub vfov_degrees: f64,
    pub aspect_ratio: f64,
    pub aperture: f64,
    pub focus_dist: f64,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            look_from: Point3::new(0.0, 1.0, 3.0),
            look_at: Point3::zero(),
            vup: Vec3::unit_y(),
            vfov_degrees: 40.0,
            aspect_ratio: 16.0 / 9.0,
            aperture: 0.0,
            focus_dist: 3.0,
        }
    }
}

impl Camera {
    /// Constructs the camera from configuration. The orthonormal basis is:
    ///   w = normalize(look_from - look_at)   (points backward, away from scene)
    ///   u = normalize(vup × w)               (points right)
    ///   v = w × u                             (points up, orthogonal to both)
    pub fn new(config: &CameraConfig) -> Self {
        let theta = config.vfov_degrees.to_radians();
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = config.aspect_ratio * viewport_height;

        let w = (config.look_from - config.look_at).normalized();
        let u = config.vup.cross(w).normalized();
        let v = w.cross(u);

        let horizontal = u * viewport_width * config.focus_dist;
        let vertical = v * viewport_height * config.focus_dist;
        let lower_left =
            config.look_from - horizontal / 2.0 - vertical / 2.0 - w * config.focus_dist;

        Camera {
            origin: config.look_from,
            lower_left,
            horizontal,
            vertical,
            u,
            v,
            lens_radius: config.aperture / 2.0,
        }
    }

    /// Generates a primary ray for the given (s, t) coordinates in [0,1]².
    /// When `lens_radius > 0`, the ray origin is perturbed for depth-of-field.
    pub fn get_ray(&self, s: f64, t: f64, rng: &mut dyn rand::RngCore) -> Ray {
        let rd = Vec3::random_in_unit_disk(rng) * self.lens_radius;
        let offset = self.u * rd.x + self.v * rd.y;
        Ray::new(
            self.origin + offset,
            self.lower_left + self.horizontal * s + self.vertical * t - self.origin - offset,
        )
    }
}
