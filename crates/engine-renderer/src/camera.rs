use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// GPU用カメラUniform
/// View-Projection行列を列優先形式で格納
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    /// Mat4からCameraUniformを作成
    pub fn from_mat4(mat: Mat4) -> Self {
        Self {
            view_proj: mat.to_cols_array_2d(),
        }
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::from_mat4(Mat4::IDENTITY)
    }
}

/// 3Dカメラ
/// 位置、注視点、上方向ベクトルを持つ透視投影カメラ
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// デフォルト値で新しいカメラを作成
    pub fn new(aspect: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 45.0_f32.to_radians(),
            aspect,
            near: 0.1,
            far: 100.0,
        }
    }

    /// View-Projection行列を構築
    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        let proj = Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far);
        proj * view
    }

    /// GPU用のCameraUniformを取得
    pub fn uniform(&self) -> CameraUniform {
        CameraUniform::from_mat4(self.build_view_projection_matrix())
    }

    /// カメラ位置を設定
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    /// カメラの注視点を設定
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }

    /// アスペクト比を設定
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_new() {
        let camera = Camera::new(16.0 / 9.0);
        assert_eq!(camera.position, Vec3::new(0.0, 2.0, 5.0));
        assert_eq!(camera.target, Vec3::ZERO);
        assert_eq!(camera.up, Vec3::Y);
    }

    #[test]
    fn test_camera_uniform_size() {
        // 4x4 floats * 4 bytes = 64 bytes
        assert_eq!(std::mem::size_of::<CameraUniform>(), 64);
    }

    #[test]
    fn test_view_projection_not_identity() {
        let camera = Camera::new(1.0);
        let vp = camera.build_view_projection_matrix();
        assert_ne!(vp, Mat4::IDENTITY);
    }

    #[test]
    fn test_uniform_creation() {
        let camera = Camera::new(1.0);
        let uniform = camera.uniform();
        // Should not be all zeros
        let has_nonzero = uniform.view_proj.iter()
            .flat_map(|row| row.iter())
            .any(|&v| v != 0.0);
        assert!(has_nonzero);
    }

    #[test]
    fn test_set_position() {
        let mut camera = Camera::new(1.0);
        camera.set_position(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(camera.position, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_set_target() {
        let mut camera = Camera::new(1.0);
        camera.set_target(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(camera.target, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_set_aspect() {
        let mut camera = Camera::new(1.0);
        camera.set_aspect(2.0);
        assert_eq!(camera.aspect, 2.0);
    }
}
