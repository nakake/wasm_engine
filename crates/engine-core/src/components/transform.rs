use crate::ecs::Component;
use crate::math::{Mat4, Quat, Vec3};
use bytemuck::{Pod, Zeroable};

/// 3D Transform component
/// Represents position, rotation and scale in 3D space
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    /// Create a new Transform with specified values
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    /// Create an identity transform (no translation, rotation, or scale)
    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Create a transform with only position
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Convert to a 4x4 transformation matrix
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Component for Transform {}

/// GPU用Model行列Uniform
/// シェーダーに渡すための4x4行列（列優先）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ModelUniform {
    pub model: [[f32; 4]; 4],
}

impl ModelUniform {
    /// TransformからModelUniformを作成
    pub fn from_transform(transform: &Transform) -> Self {
        Self {
            model: transform.to_matrix().to_cols_array_2d(),
        }
    }

    /// 単位行列のModelUniformを作成
    pub fn identity() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

impl Default for ModelUniform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let t = Transform::identity();
        assert_eq!(t.position, Vec3::ZERO);
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_from_position() {
        let t = Transform::from_position(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_to_matrix_identity() {
        let t = Transform::identity();
        let m = t.to_matrix();
        assert_eq!(m, Mat4::IDENTITY);
    }

    #[test]
    fn test_to_matrix_translation() {
        let t = Transform::from_position(Vec3::new(1.0, 2.0, 3.0));
        let m = t.to_matrix();
        let expected = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(m, expected);
    }

    #[test]
    fn test_to_matrix_scale() {
        let t = Transform {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::new(2.0, 3.0, 4.0),
        };
        let m = t.to_matrix();
        // 対角成分がスケール値
        assert_eq!(m.x_axis.x, 2.0);
        assert_eq!(m.y_axis.y, 3.0);
        assert_eq!(m.z_axis.z, 4.0);
    }

    #[test]
    fn test_model_uniform_size() {
        // 4x4 floats * 4 bytes = 64 bytes
        assert_eq!(std::mem::size_of::<ModelUniform>(), 64);
    }

    #[test]
    fn test_model_uniform_from_identity_transform() {
        let t = Transform::identity();
        let uniform = ModelUniform::from_transform(&t);
        let identity = ModelUniform::identity();
        assert_eq!(uniform.model, identity.model);
    }

    #[test]
    fn test_model_uniform_from_translated_transform() {
        let t = Transform::from_position(Vec3::new(1.0, 2.0, 3.0));
        let uniform = ModelUniform::from_transform(&t);
        // 列優先なので、w列（4列目）がtranslation
        assert_eq!(uniform.model[3][0], 1.0);
        assert_eq!(uniform.model[3][1], 2.0);
        assert_eq!(uniform.model[3][2], 3.0);
        assert_eq!(uniform.model[3][3], 1.0);
    }

    #[test]
    fn test_model_uniform_default() {
        let uniform = ModelUniform::default();
        let identity = ModelUniform::identity();
        assert_eq!(uniform.model, identity.model);
    }
}
