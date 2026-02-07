use rand::Rng;
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Index, Mul, MulAssign, Neg, Sub};

/// A 3-component vector used for positions, directions, and colors in the ray tracer.
///
/// This type implements all standard arithmetic operations with operator overloading,
/// and provides geometric utilities (dot product, cross product, reflection, refraction)
/// needed for physically-based light transport simulation.
#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub type Point3 = Vec3;
pub type Color = Vec3;

impl Vec3 {
    #[inline(always)]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    #[inline(always)]
    pub const fn ones() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }

    #[inline(always)]
    pub const fn unit_x() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }

    #[inline(always)]
    pub const fn unit_y() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }

    #[inline(always)]
    pub const fn unit_z() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }

    /// Squared Euclidean length — avoids the sqrt for performance-critical paths
    /// such as BVH traversal and intersection culling.
    #[inline(always)]
    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline(always)]
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Returns the unit vector. Panics on zero-length vectors in debug mode.
    #[inline(always)]
    pub fn normalized(self) -> Self {
        let len = self.length();
        debug_assert!(len > 1e-12, "Attempted to normalize a zero-length vector");
        self / len
    }

    /// The standard Euclidean inner product, fundamental to all geometric queries
    /// in the ray tracer (projection, angle computation, Lambertian shading).
    #[inline(always)]
    pub fn dot(self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    /// Cross product — used for constructing orthonormal camera bases and computing
    /// surface tangent frames for normal mapping.
    #[inline(always)]
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    /// Specular reflection of `self` about the surface normal `n`.
    /// Implements the GLSL `reflect` formula: I - 2·dot(I, N)·N
    #[inline(always)]
    pub fn reflect(self, normal: Self) -> Self {
        self - normal * 2.0 * self.dot(normal)
    }

    /// Snell's law refraction. Returns `None` for total internal reflection (TIR)
    /// when the discriminant is negative, which occurs at grazing angles when
    /// transitioning from a denser to a rarer medium (η > 1).
    #[inline]
    pub fn refract(self, normal: Self, eta_ratio: f64) -> Option<Self> {
        let cos_theta = (-self).dot(normal).min(1.0);
        let r_perp = (self + normal * cos_theta) * eta_ratio;
        let discriminant = 1.0 - r_perp.length_squared();
        if discriminant < 0.0 {
            return None;
        }
        let r_parallel = normal * -(discriminant.sqrt());
        Some(r_perp + r_parallel)
    }

    /// Component-wise (Hadamard) product — used for color modulation where each
    /// channel is attenuated independently by the surface albedo.
    #[inline(always)]
    pub fn hadamard(self, rhs: Self) -> Self {
        Self::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }

    /// Component-wise linear interpolation: self·(1-t) + other·t
    #[inline(always)]
    pub fn lerp(self, other: Self, t: f64) -> Self {
        self * (1.0 - t) + other * t
    }

    /// Clamps each component to [0, 1] — used before quantizing HDR radiance values
    /// to 8-bit sRGB for terminal display.
    #[inline(always)]
    pub fn saturate(self) -> Self {
        Self::new(
            self.x.clamp(0.0, 1.0),
            self.y.clamp(0.0, 1.0),
            self.z.clamp(0.0, 1.0),
        )
    }

    /// Applies the sRGB gamma curve (γ = 2.2 approximated as sqrt) for perceptually
    /// correct display on standard monitors / terminals with true-color support.
    #[inline(always)]
    pub fn gamma_correct(self) -> Self {
        Self::new(self.x.sqrt(), self.y.sqrt(), self.z.sqrt())
    }

    /// Checks if the vector is near-zero in all components, used to avoid
    /// degenerate scatter directions that would produce NaN in subsequent math.
    #[inline(always)]
    pub fn near_zero(self) -> bool {
        const EPS: f64 = 1e-8;
        self.x.abs() < EPS && self.y.abs() < EPS && self.z.abs() < EPS
    }

    /// Converts a [0,1] color to an 8-bit RGB triple for ANSI true-color output.
    pub fn to_rgb8(self) -> (u8, u8, u8) {
        let c = self.saturate();
        (
            (c.x * 255.999) as u8,
            (c.y * 255.999) as u8,
            (c.z * 255.999) as u8,
        )
    }

    /// Generates a uniformly distributed random point inside the unit sphere
    /// via rejection sampling. Used for Lambertian diffuse scattering.
    pub fn random_in_unit_sphere(rng: &mut dyn rand::RngCore) -> Self {
        loop {
            let v = Self::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            );
            if v.length_squared() < 1.0 {
                return v;
            }
        }
    }

    /// Cosine-weighted hemisphere sampling via rejection + normalization.
    /// Produces directions distributed proportionally to cos(θ), which is
    /// the optimal importance sampling strategy for Lambertian BRDFs.
    /// Generates a random unit vector via rejection sampling on the unit sphere.
    pub fn random_unit_vector(rng: &mut dyn rand::RngCore) -> Self {
        Self::random_in_unit_sphere(rng).normalized()
    }

    /// Random point on the unit disk — used for depth-of-field simulation
    /// by jittering the camera ray origin across the lens aperture.
    pub fn random_in_unit_disk(rng: &mut dyn rand::RngCore) -> Self {
        loop {
            let v = Self::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 0.0);
            if v.length_squared() < 1.0 {
                return v;
            }
        }
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.3}, {:.3}, {:.3})", self.x, self.y, self.z)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, t: f64) -> Self {
        Self::new(self.x * t, self.y * t, self.z * t)
    }
}

impl Mul<Vec3> for f64 {
    type Output = Vec3;
    #[inline(always)]
    fn mul(self, v: Vec3) -> Vec3 {
        v * self
    }
}

impl MulAssign<f64> for Vec3 {
    #[inline(always)]
    fn mul_assign(&mut self, t: f64) {
        self.x *= t;
        self.y *= t;
        self.z *= t;
    }
}

impl Div<f64> for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn div(self, t: f64) -> Self {
        let inv = 1.0 / t;
        Self::new(self.x * inv, self.y * inv, self.z * inv)
    }
}

impl DivAssign<f64> for Vec3 {
    #[inline(always)]
    fn div_assign(&mut self, t: f64) {
        let inv = 1.0 / t;
        self.x *= inv;
        self.y *= inv;
        self.z *= inv;
    }
}

impl Index<usize> for Vec3 {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Vec3 index out of bounds: {i}"),
        }
    }
}

// ─── Ray ────────────────────────────────────────────────────────────────────

/// A parametric ray R(t) = origin + t · direction, the fundamental geometric
/// primitive for all intersection queries in the path tracer.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Vec3,
}

impl Ray {
    #[inline(always)]
    pub const fn new(origin: Point3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    /// Evaluates the ray at parameter t. Positive t gives points ahead of the origin.
    #[inline(always)]
    pub fn at(self, t: f64) -> Point3 {
        self.origin + self.direction * t
    }
}

// ─── Axis-Aligned Bounding Box ──────────────────────────────────────────────

/// An axis-aligned bounding box (AABB) used as the bounding volume in the BVH.
/// Intersection is tested via the slab method, which checks overlap of the ray's
/// parameter intervals across all three axes simultaneously.
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb {
    pub const fn new(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }

    /// Slab-method ray-AABB intersection test. Returns true if the ray hits the box
    /// within [t_min, t_max]. The branchless min/max formulation handles NaN and
    /// axis-aligned rays correctly.
    pub fn hit(&self, ray: &Ray, mut t_min: f64, mut t_max: f64) -> bool {
        for axis in 0..3 {
            let inv_d = 1.0 / ray.direction[axis];
            let mut t0 = (self.min[axis] - ray.origin[axis]) * inv_d;
            let mut t1 = (self.max[axis] - ray.origin[axis]) * inv_d;
            if inv_d < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t0.max(t_min);
            t_max = t1.min(t_max);
            if t_max <= t_min {
                return false;
            }
        }
        true
    }

    /// Computes the union of two AABBs — used during BVH construction to find
    /// the bounding volume of a set of child nodes.
    pub fn surrounding(a: &Aabb, b: &Aabb) -> Aabb {
        let min = Point3::new(
            a.min.x.min(b.min.x),
            a.min.y.min(b.min.y),
            a.min.z.min(b.min.z),
        );
        let max = Point3::new(
            a.max.x.max(b.max.x),
            a.max.y.max(b.max.y),
            a.max.z.max(b.max.z),
        );
        Aabb::new(min, max)
    }

    /// Returns the index of the longest axis (0=x, 1=y, 2=z) — used as the
    /// split dimension during top-down BVH construction with the midpoint heuristic.
    pub fn longest_axis(&self) -> usize {
        let dx = self.max.x - self.min.x;
        let dy = self.max.y - self.min.y;
        let dz = self.max.z - self.min.z;
        if dx > dy && dx > dz {
            0
        } else if dy > dz {
            1
        } else {
            2
        }
    }
}
