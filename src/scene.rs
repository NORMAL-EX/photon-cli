use crate::math::*;
use rand::Rng;
use std::cmp::Ordering;

// ─── Hit Record ─────────────────────────────────────────────────────────────

pub struct HitRecord<'a> {
    pub point: Point3,
    pub normal: Vec3,
    pub t: f64,
    pub front_face: bool,
    pub material: &'a dyn Material,
}

impl<'a> HitRecord<'a> {
    pub fn set_face_normal(&mut self, ray: &Ray, outward_normal: Vec3) {
        self.front_face = ray.direction.dot(outward_normal) < 0.0;
        self.normal = if self.front_face {
            outward_normal
        } else {
            -outward_normal
        };
    }
}

// ─── Material Trait ─────────────────────────────────────────────────────────

pub trait Material: Send + Sync {
    fn scatter(
        &self,
        ray: &Ray,
        hit: &HitRecord,
        rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Color)>;

    fn emitted(&self) -> Color {
        Color::zero()
    }
}

// ─── Lambertian (Diffuse) ───────────────────────────────────────────────────

pub struct Lambertian {
    pub albedo: Color,
}

impl Lambertian {
    pub const fn new(albedo: Color) -> Self {
        Self { albedo }
    }
}

impl Material for Lambertian {
    fn scatter(
        &self,
        _ray: &Ray,
        hit: &HitRecord,
        rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Color)> {
        let mut scatter_dir = hit.normal + Vec3::random_unit_vector(rng);
        if scatter_dir.near_zero() {
            scatter_dir = hit.normal;
        }
        Some((Ray::new(hit.point, scatter_dir), self.albedo))
    }
}

// ─── Metal (Specular) ───────────────────────────────────────────────────────

pub struct Metal {
    pub albedo: Color,
    pub fuzz: f64,
}

impl Metal {
    pub fn new(albedo: Color, fuzz: f64) -> Self {
        Self {
            albedo,
            fuzz: fuzz.min(1.0),
        }
    }
}

impl Material for Metal {
    fn scatter(
        &self,
        ray: &Ray,
        hit: &HitRecord,
        rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Color)> {
        let reflected = ray.direction.normalized().reflect(hit.normal);
        let scattered = Ray::new(
            hit.point,
            reflected + Vec3::random_in_unit_sphere(rng) * self.fuzz,
        );
        if scattered.direction.dot(hit.normal) > 0.0 {
            Some((scattered, self.albedo))
        } else {
            None
        }
    }
}

// ─── Dielectric (Glass) ────────────────────────────────────────────────────

pub struct Dielectric {
    pub ior: f64,
}

impl Dielectric {
    pub const fn new(ior: f64) -> Self {
        Self { ior }
    }

    /// Schlick's approximation for Fresnel reflectance at grazing angles.
    fn schlick_reflectance(cosine: f64, ref_idx: f64) -> f64 {
        let r0 = ((1.0 - ref_idx) / (1.0 + ref_idx)).powi(2);
        r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
    }
}

impl Material for Dielectric {
    fn scatter(
        &self,
        ray: &Ray,
        hit: &HitRecord,
        rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Color)> {
        let eta_ratio = if hit.front_face {
            1.0 / self.ior
        } else {
            self.ior
        };
        let unit_dir = ray.direction.normalized();
        let cos_theta = (-unit_dir).dot(hit.normal).min(1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let cannot_refract = eta_ratio * sin_theta > 1.0;
        let direction =
            if cannot_refract || Self::schlick_reflectance(cos_theta, eta_ratio) > rng.gen() {
                unit_dir.reflect(hit.normal)
            } else {
                unit_dir
                    .refract(hit.normal, eta_ratio)
                    .unwrap_or_else(|| unit_dir.reflect(hit.normal))
            };

        Some((Ray::new(hit.point, direction), Color::ones()))
    }
}

// ─── Emissive Material ──────────────────────────────────────────────────────

pub struct Emissive {
    pub emit_color: Color,
    pub intensity: f64,
}

impl Emissive {
    pub const fn new(emit_color: Color, intensity: f64) -> Self {
        Self {
            emit_color,
            intensity,
        }
    }
}

impl Material for Emissive {
    fn scatter(
        &self,
        _ray: &Ray,
        _hit: &HitRecord,
        _rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Color)> {
        None
    }

    fn emitted(&self) -> Color {
        self.emit_color * self.intensity
    }
}

// ─── Checkerboard Material ──────────────────────────────────────────────────

pub struct Checkerboard {
    pub color_a: Color,
    pub color_b: Color,
    pub scale: f64,
}

impl Checkerboard {
    pub fn new(color_a: Color, color_b: Color, scale: f64) -> Self {
        Self {
            color_a,
            color_b,
            scale,
        }
    }

    fn pattern_at(&self, point: Point3) -> Color {
        let sines = (self.scale * point.x).sin()
            * (self.scale * point.y).sin()
            * (self.scale * point.z).sin();
        if sines < 0.0 {
            self.color_a
        } else {
            self.color_b
        }
    }
}

impl Material for Checkerboard {
    fn scatter(
        &self,
        _ray: &Ray,
        hit: &HitRecord,
        rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Color)> {
        let mut scatter_dir = hit.normal + Vec3::random_unit_vector(rng);
        if scatter_dir.near_zero() {
            scatter_dir = hit.normal;
        }
        Some((Ray::new(hit.point, scatter_dir), self.pattern_at(hit.point)))
    }
}

// ─── Gradient Material ──────────────────────────────────────────────────────

/// A procedural material that smoothly interpolates between two colors based on surface
/// normal orientation. Produces a smooth gradient effect driven by the dot
/// product between the hit normal and a configurable axis direction.
pub struct GradientMaterial {
    pub color_a: Color,
    pub color_b: Color,
    pub axis: Vec3,
}

impl GradientMaterial {
    pub fn new(color_a: Color, color_b: Color, axis: Vec3) -> Self {
        Self {
            color_a,
            color_b,
            axis: axis.normalized(),
        }
    }
}

impl Material for GradientMaterial {
    fn scatter(
        &self,
        _ray: &Ray,
        hit: &HitRecord,
        rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Color)> {
        let mut scatter_dir = hit.normal + Vec3::random_unit_vector(rng);
        if scatter_dir.near_zero() {
            scatter_dir = hit.normal;
        }
        let t = (hit.normal.dot(self.axis) * 0.5 + 0.5).clamp(0.0, 1.0);
        let albedo = self.color_a.lerp(self.color_b, t);
        Some((Ray::new(hit.point, scatter_dir), albedo))
    }
}

// ─── Hittable Trait ─────────────────────────────────────────────────────────

pub trait Hittable: Send + Sync {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord<'_>>;
    fn bounding_box(&self) -> Aabb;
}

// ─── Sphere ─────────────────────────────────────────────────────────────────

pub struct Sphere {
    pub center: Point3,
    pub radius: f64,
    pub material: Box<dyn Material>,
}

impl Sphere {
    pub fn new(center: Point3, radius: f64, material: impl Material + 'static) -> Self {
        Self {
            center,
            radius,
            material: Box::new(material),
        }
    }
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord<'_>> {
        let oc = ray.origin - self.center;
        let a = ray.direction.length_squared();
        let half_b = oc.dot(ray.direction);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrtd = discriminant.sqrt();
        let mut root = (-half_b - sqrtd) / a;
        if root < t_min || root > t_max {
            root = (-half_b + sqrtd) / a;
            if root < t_min || root > t_max {
                return None;
            }
        }

        let point = ray.at(root);
        let outward_normal = (point - self.center) / self.radius;
        let mut rec = HitRecord {
            point,
            normal: outward_normal,
            t: root,
            front_face: true,
            material: self.material.as_ref(),
        };
        rec.set_face_normal(ray, outward_normal);
        Some(rec)
    }

    fn bounding_box(&self) -> Aabb {
        let r = Vec3::new(self.radius.abs(), self.radius.abs(), self.radius.abs());
        Aabb::new(self.center - r, self.center + r)
    }
}

// ─── Infinite Plane ─────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct Plane {
    pub point: Point3,
    pub normal: Vec3,
    pub material: Box<dyn Material>,
}

impl Plane {
    #[allow(dead_code)]
    pub fn new(point: Point3, normal: Vec3, material: impl Material + 'static) -> Self {
        Self {
            point,
            normal: normal.normalized(),
            material: Box::new(material),
        }
    }
}

impl Hittable for Plane {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord<'_>> {
        let denom = ray.direction.dot(self.normal);
        if denom.abs() < 1e-8 {
            return None;
        }
        let t = (self.point - ray.origin).dot(self.normal) / denom;
        if t < t_min || t > t_max {
            return None;
        }
        let point = ray.at(t);
        let mut rec = HitRecord {
            point,
            normal: self.normal,
            t,
            front_face: true,
            material: self.material.as_ref(),
        };
        rec.set_face_normal(ray, self.normal);
        Some(rec)
    }

    fn bounding_box(&self) -> Aabb {
        let big = 1e4;
        Aabb::new(Point3::new(-big, -big, -big), Point3::new(big, big, big))
    }
}

// ─── Triangle (Möller–Trumbore) ─────────────────────────────────────────────

#[allow(dead_code)]
pub struct Triangle {
    pub v0: Point3,
    pub v1: Point3,
    pub v2: Point3,
    pub material: Box<dyn Material>,
}

impl Triangle {
    #[allow(dead_code)]
    pub fn new(v0: Point3, v1: Point3, v2: Point3, material: impl Material + 'static) -> Self {
        Self {
            v0,
            v1,
            v2,
            material: Box::new(material),
        }
    }
}

impl Hittable for Triangle {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord<'_>> {
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        let h = ray.direction.cross(edge2);
        let a = edge1.dot(h);
        if a.abs() < 1e-8 {
            return None;
        }

        let f = 1.0 / a;
        let s = ray.origin - self.v0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * ray.direction.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);
        if t < t_min || t > t_max {
            return None;
        }

        let point = ray.at(t);
        let outward_normal = edge1.cross(edge2).normalized();
        let mut rec = HitRecord {
            point,
            normal: outward_normal,
            t,
            front_face: true,
            material: self.material.as_ref(),
        };
        rec.set_face_normal(ray, outward_normal);
        Some(rec)
    }

    fn bounding_box(&self) -> Aabb {
        let eps = 1e-4;
        let min = Point3::new(
            self.v0.x.min(self.v1.x).min(self.v2.x) - eps,
            self.v0.y.min(self.v1.y).min(self.v2.y) - eps,
            self.v0.z.min(self.v1.z).min(self.v2.z) - eps,
        );
        let max = Point3::new(
            self.v0.x.max(self.v1.x).max(self.v2.x) + eps,
            self.v0.y.max(self.v1.y).max(self.v2.y) + eps,
            self.v0.z.max(self.v1.z).max(self.v2.z) + eps,
        );
        Aabb::new(min, max)
    }
}

// ─── Axis-Aligned Quad (Rectangle) ─────────────────────────────────────────

/// A finite rectangle defined by two edge vectors and an origin point.
/// Parameterized as: P = origin + u·edge_u + v·edge_v, for (u, v) ∈ [0,1]².
///
/// Hit detection: implicit plane equation followed by parametric bounds check on (u,v).
pub struct Quad {
    pub origin: Point3,
    pub edge_u: Vec3,
    pub edge_v: Vec3,
    pub normal: Vec3,
    pub d: f64,
    pub w: Vec3,
    pub material: Box<dyn Material>,
}

impl Quad {
    pub fn new(
        origin: Point3,
        edge_u: Vec3,
        edge_v: Vec3,
        material: impl Material + 'static,
    ) -> Self {
        let n = edge_u.cross(edge_v);
        let normal = n.normalized();
        let d = normal.dot(origin);
        let w = n / n.dot(n);
        Self {
            origin,
            edge_u,
            edge_v,
            normal,
            d,
            w,
            material: Box::new(material),
        }
    }
}

impl Hittable for Quad {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord<'_>> {
        let denom = self.normal.dot(ray.direction);
        if denom.abs() < 1e-8 {
            return None;
        }

        let t = (self.d - self.normal.dot(ray.origin)) / denom;
        if t < t_min || t > t_max {
            return None;
        }

        let intersection = ray.at(t);
        let planar_hitpt = intersection - self.origin;
        let alpha = self.w.dot(planar_hitpt.cross(self.edge_v));
        let beta = self.w.dot(self.edge_u.cross(planar_hitpt));

        if !(0.0..=1.0).contains(&alpha) || !(0.0..=1.0).contains(&beta) {
            return None;
        }

        let mut rec = HitRecord {
            point: intersection,
            normal: self.normal,
            t,
            front_face: true,
            material: self.material.as_ref(),
        };
        rec.set_face_normal(ray, self.normal);
        Some(rec)
    }

    fn bounding_box(&self) -> Aabb {
        let eps = Vec3::new(1e-4, 1e-4, 1e-4);
        let p0 = self.origin;
        let p1 = self.origin + self.edge_u;
        let p2 = self.origin + self.edge_v;
        let p3 = self.origin + self.edge_u + self.edge_v;
        let min = Point3::new(
            p0.x.min(p1.x).min(p2.x).min(p3.x),
            p0.y.min(p1.y).min(p2.y).min(p3.y),
            p0.z.min(p1.z).min(p2.z).min(p3.z),
        );
        let max = Point3::new(
            p0.x.max(p1.x).max(p2.x).max(p3.x),
            p0.y.max(p1.y).max(p2.y).max(p3.y),
            p0.z.max(p1.z).max(p2.z).max(p3.z),
        );
        Aabb::new(min - eps, max + eps)
    }
}

// ─── Disk ───────────────────────────────────────────────────────────────────

/// A circular disk primitive defined by center, normal, and radius. Ray-plane intersection followed by radius check.
pub struct Disk {
    pub center: Point3,
    pub normal: Vec3,
    pub radius: f64,
    pub material: Box<dyn Material>,
}

impl Disk {
    pub fn new(
        center: Point3,
        normal: Vec3,
        radius: f64,
        material: impl Material + 'static,
    ) -> Self {
        Self {
            center,
            normal: normal.normalized(),
            radius,
            material: Box::new(material),
        }
    }
}

impl Hittable for Disk {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord<'_>> {
        let denom = ray.direction.dot(self.normal);
        if denom.abs() < 1e-8 {
            return None;
        }
        let t = (self.center - ray.origin).dot(self.normal) / denom;
        if t < t_min || t > t_max {
            return None;
        }
        let point = ray.at(t);
        let dist_sq = (point - self.center).length_squared();
        if dist_sq > self.radius * self.radius {
            return None;
        }
        let mut rec = HitRecord {
            point,
            normal: self.normal,
            t,
            front_face: true,
            material: self.material.as_ref(),
        };
        rec.set_face_normal(ray, self.normal);
        Some(rec)
    }

    fn bounding_box(&self) -> Aabb {
        let r = Vec3::new(self.radius, self.radius, self.radius);
        Aabb::new(self.center - r, self.center + r)
    }
}

// ─── Bounding Volume Hierarchy ──────────────────────────────────────────────

pub enum BvhNode {
    Leaf {
        object: Box<dyn Hittable>,
        bbox: Aabb,
    },
    Interior {
        left: Box<BvhNode>,
        right: Box<BvhNode>,
        bbox: Aabb,
    },
}

impl BvhNode {
    pub fn build(mut objects: Vec<Box<dyn Hittable>>) -> Self {
        let len = objects.len();
        match len {
            0 => panic!("BVH: empty object list"),
            1 => {
                let obj = objects.pop().unwrap();
                let bbox = obj.bounding_box();
                BvhNode::Leaf { object: obj, bbox }
            }
            _ => {
                let enclosing = objects
                    .iter()
                    .map(|o| o.bounding_box())
                    .reduce(|a, b| Aabb::surrounding(&a, &b))
                    .unwrap();
                let axis = enclosing.longest_axis();

                objects.sort_by(|a, b| {
                    let ac = a.bounding_box().min[axis] + a.bounding_box().max[axis];
                    let bc = b.bounding_box().min[axis] + b.bounding_box().max[axis];
                    ac.partial_cmp(&bc).unwrap_or(Ordering::Equal)
                });

                let mid = len / 2;
                let right_objs = objects.split_off(mid);
                let left = Box::new(BvhNode::build(objects));
                let right = Box::new(BvhNode::build(right_objs));
                let bbox =
                    Aabb::surrounding(&left.bounding_box_inner(), &right.bounding_box_inner());
                BvhNode::Interior { left, right, bbox }
            }
        }
    }

    fn bounding_box_inner(&self) -> Aabb {
        match self {
            BvhNode::Leaf { bbox, .. } => *bbox,
            BvhNode::Interior { bbox, .. } => *bbox,
        }
    }

    /// Returns the total number of leaf (primitive) nodes in the BVH.
    pub fn leaf_count(&self) -> usize {
        match self {
            BvhNode::Leaf { .. } => 1,
            BvhNode::Interior { left, right, .. } => left.leaf_count() + right.leaf_count(),
        }
    }

    /// Returns the maximum depth of the BVH tree.
    pub fn depth(&self) -> usize {
        match self {
            BvhNode::Leaf { .. } => 1,
            BvhNode::Interior { left, right, .. } => 1 + left.depth().max(right.depth()),
        }
    }
}

impl Hittable for BvhNode {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord<'_>> {
        match self {
            BvhNode::Leaf { object, bbox } => {
                if !bbox.hit(ray, t_min, t_max) {
                    return None;
                }
                object.hit(ray, t_min, t_max)
            }
            BvhNode::Interior {
                left, right, bbox, ..
            } => {
                if !bbox.hit(ray, t_min, t_max) {
                    return None;
                }
                let hit_left = left.hit(ray, t_min, t_max);
                let far = hit_left.as_ref().map_or(t_max, |h| h.t);
                let hit_right = right.hit(ray, t_min, far);
                hit_right.or(hit_left)
            }
        }
    }

    fn bounding_box(&self) -> Aabb {
        self.bounding_box_inner()
    }
}
