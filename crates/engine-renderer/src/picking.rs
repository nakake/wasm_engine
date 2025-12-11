//! Picking system for ray-AABB intersection
//!
//! Provides CPU-based entity picking using raycasting.

use glam::Vec3;

/// Ray for raycasting
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// Ray origin point
    pub origin: Vec3,
    /// Ray direction (should be normalized)
    pub direction: Vec3,
}

impl Ray {
    /// Create a new ray
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Check intersection with AABB using slab method
    /// Returns the distance to intersection point if hit, None otherwise
    pub fn intersect_aabb(&self, aabb: &AABB) -> Option<f32> {
        // Handle division by zero with small epsilon
        let inv_dir = Vec3::new(
            if self.direction.x.abs() > f32::EPSILON {
                1.0 / self.direction.x
            } else {
                f32::MAX
            },
            if self.direction.y.abs() > f32::EPSILON {
                1.0 / self.direction.y
            } else {
                f32::MAX
            },
            if self.direction.z.abs() > f32::EPSILON {
                1.0 / self.direction.z
            } else {
                f32::MAX
            },
        );

        let t1 = (aabb.min - self.origin) * inv_dir;
        let t2 = (aabb.max - self.origin) * inv_dir;

        let tmin_vec = t1.min(t2);
        let tmax_vec = t1.max(t2);

        let tmin = tmin_vec.x.max(tmin_vec.y).max(tmin_vec.z);
        let tmax = tmax_vec.x.min(tmax_vec.y).min(tmax_vec.z);

        if tmax >= tmin && tmax >= 0.0 {
            Some(tmin.max(0.0))
        } else {
            None
        }
    }
}

/// Axis-Aligned Bounding Box
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    /// Minimum corner
    pub min: Vec3,
    /// Maximum corner
    pub max: Vec3,
}

impl AABB {
    /// Create a new AABB from min and max corners
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Create a unit cube centered at position with given scale
    pub fn unit_cube(center: Vec3, scale: Vec3) -> Self {
        let half = scale * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// Check if a point is inside the AABB
    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_aabb_hit() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let aabb = AABB::unit_cube(Vec3::ZERO, Vec3::ONE);

        let result = ray.intersect_aabb(&aabb);
        assert!(result.is_some());
        let t = result.unwrap();
        assert!((t - 4.5).abs() < 0.001); // Should hit at z = 0.5
    }

    #[test]
    fn test_ray_aabb_miss() {
        let ray = Ray::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let aabb = AABB::unit_cube(Vec3::ZERO, Vec3::ONE);

        let result = ray.intersect_aabb(&aabb);
        assert!(result.is_none());
    }

    #[test]
    fn test_ray_inside_aabb() {
        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        let aabb = AABB::unit_cube(Vec3::ZERO, Vec3::ONE);

        let result = ray.intersect_aabb(&aabb);
        assert!(result.is_some());
        assert!(result.unwrap() >= 0.0);
    }

    #[test]
    fn test_aabb_contains() {
        let aabb = AABB::unit_cube(Vec3::ZERO, Vec3::ONE);

        assert!(aabb.contains(Vec3::ZERO));
        assert!(aabb.contains(Vec3::new(0.4, 0.4, 0.4)));
        assert!(!aabb.contains(Vec3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_unit_cube() {
        let aabb = AABB::unit_cube(Vec3::new(1.0, 2.0, 3.0), Vec3::new(2.0, 4.0, 6.0));

        assert_eq!(aabb.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(aabb.max, Vec3::new(2.0, 4.0, 6.0));
    }
}
