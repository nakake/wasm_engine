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

/// Orbitカメラ
/// ターゲットを中心に回転するカメラ（エディタ向け）
#[derive(Debug, Clone)]
pub struct Camera {
    /// カメラ位置（計算で導出）
    position: Vec3,
    /// 注視点
    target: Vec3,
    /// 上方向ベクトル
    up: Vec3,
    /// ターゲットからの距離
    distance: f32,
    /// 水平角度（ラジアン）
    yaw: f32,
    /// 垂直角度（ラジアン）
    pitch: f32,
    /// 最小距離
    min_distance: f32,
    /// 最大距離
    max_distance: f32,
    /// 視野角
    fov: f32,
    /// アスペクト比
    aspect: f32,
    /// ニアクリップ
    near: f32,
    /// ファークリップ
    far: f32,
}

impl Camera {
    /// デフォルト値で新しいOrbitカメラを作成
    pub fn new(aspect: f32) -> Self {
        let mut camera = Self {
            position: Vec3::ZERO,
            target: Vec3::ZERO,
            up: Vec3::Y,
            distance: 5.0,
            yaw: 0.0,
            pitch: 0.4, // 約23度上から見下ろす
            min_distance: 0.5,
            max_distance: 100.0,
            fov: 45.0_f32.to_radians(),
            aspect,
            near: 0.1,
            far: 100.0,
        };
        camera.update_position();
        camera
    }

    /// カメラをターゲット周りで回転（Orbit）
    ///
    /// # Arguments
    /// * `delta_x` - 水平方向の回転量（ラジアン）
    /// * `delta_y` - 垂直方向の回転量（ラジアン）
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x;
        // Pitchを-89度〜89度に制限（真上/真下を防ぐ）
        self.pitch = (self.pitch + delta_y).clamp(
            -89.0_f32.to_radians(),
            89.0_f32.to_radians(),
        );
        self.update_position();
    }

    /// カメラを平行移動（Pan）
    ///
    /// # Arguments
    /// * `delta_x` - 右方向への移動量
    /// * `delta_y` - 上方向への移動量
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.position).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward).normalize();

        // 距離に応じてパン速度を調整
        let scale = self.distance * 0.5;
        self.target += right * delta_x * scale + up * delta_y * scale;
        self.update_position();
    }

    /// カメラをズーム
    ///
    /// # Arguments
    /// * `delta` - 正で近づく、負で遠ざかる
    pub fn zoom(&mut self, delta: f32) {
        // 対数スケールでズーム（近いときは細かく、遠いときは大きく）
        let zoom_factor = 1.0 - delta * 0.1;
        self.distance = (self.distance * zoom_factor).clamp(self.min_distance, self.max_distance);
        self.update_position();
    }

    /// ターゲットからの距離を設定
    pub fn set_distance(&mut self, distance: f32) {
        self.distance = distance.clamp(self.min_distance, self.max_distance);
        self.update_position();
    }

    /// 球面座標からカメラ位置を更新
    fn update_position(&mut self) {
        // 球面座標 -> デカルト座標
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.position = self.target + Vec3::new(x, y, z);
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

    /// カメラ位置を取得
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// カメラの注視点を取得
    pub fn target(&self) -> Vec3 {
        self.target
    }

    /// カメラの注視点を設定
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
        self.update_position();
    }

    /// アスペクト比を設定
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// 特定の位置にフォーカス（ターゲットを移動）
    pub fn focus_on(&mut self, position: Vec3) {
        self.target = position;
        self.update_position();
    }

    /// Yaw角度を取得
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Pitch角度を取得
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// 距離を取得
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// View行列を取得
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Projection行列を取得
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }

    /// スクリーン座標（0〜1）からワールド空間のレイを生成
    ///
    /// # Arguments
    /// * `screen_x` - スクリーンX座標 (0.0 = 左端, 1.0 = 右端)
    /// * `screen_y` - スクリーンY座標 (0.0 = 上端, 1.0 = 下端)
    pub fn screen_to_ray(&self, screen_x: f32, screen_y: f32) -> crate::picking::Ray {
        // NDC座標に変換 (-1〜1)
        let ndc_x = screen_x * 2.0 - 1.0;
        let ndc_y = 1.0 - screen_y * 2.0; // Y反転

        // ビュー空間でのレイ方向を計算
        let tan_fov = (self.fov * 0.5).tan();
        let view_dir = Vec3::new(
            ndc_x * self.aspect * tan_fov,
            ndc_y * tan_fov,
            -1.0,
        )
        .normalize();

        // ワールド空間に変換
        let view_matrix = self.view_matrix();
        let inv_view = view_matrix.inverse();

        let world_dir = inv_view.transform_vector3(view_dir).normalize();

        crate::picking::Ray::new(self.position, world_dir)
    }

    /// スクリーン座標をワールド座標に変換
    ///
    /// # Arguments
    /// * `screen_x` - スクリーンX座標 (0.0〜1.0)
    /// * `screen_y` - スクリーンY座標 (0.0〜1.0)
    /// * `depth` - 深度値 (0.0 = near, 1.0 = far)
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32, depth: f32) -> Vec3 {
        // NDC座標に変換
        let ndc_x = screen_x * 2.0 - 1.0;
        let ndc_y = 1.0 - screen_y * 2.0;
        let ndc_z = depth * 2.0 - 1.0;

        // 逆VP行列を計算
        let vp = self.build_view_projection_matrix();
        let inv_vp = vp.inverse();

        // NDCからワールドへ
        let clip_pos = glam::Vec4::new(ndc_x, ndc_y, ndc_z, 1.0);
        let world_pos = inv_vp * clip_pos;

        Vec3::new(
            world_pos.x / world_pos.w,
            world_pos.y / world_pos.w,
            world_pos.z / world_pos.w,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_new() {
        let camera = Camera::new(16.0 / 9.0);
        assert_eq!(camera.target, Vec3::ZERO);
        assert_eq!(camera.up, Vec3::Y);
        assert_eq!(camera.distance, 5.0);
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
        let has_nonzero = uniform
            .view_proj
            .iter()
            .flat_map(|row| row.iter())
            .any(|&v| v != 0.0);
        assert!(has_nonzero);
    }

    #[test]
    fn test_orbit() {
        let mut camera = Camera::new(1.0);
        let initial_pos = camera.position();
        camera.orbit(0.5, 0.0);
        assert_ne!(camera.position(), initial_pos);
        assert_eq!(camera.yaw(), 0.5);
    }

    #[test]
    fn test_orbit_pitch_clamp() {
        let mut camera = Camera::new(1.0);
        // Try to pitch beyond 89 degrees
        camera.orbit(0.0, std::f32::consts::PI);
        assert!(camera.pitch() <= 89.0_f32.to_radians());
        assert!(camera.pitch() >= -89.0_f32.to_radians());
    }

    #[test]
    fn test_pan() {
        let mut camera = Camera::new(1.0);
        let initial_target = camera.target();
        camera.pan(1.0, 0.0);
        assert_ne!(camera.target(), initial_target);
    }

    #[test]
    fn test_zoom() {
        let mut camera = Camera::new(1.0);
        let initial_distance = camera.distance();
        camera.zoom(1.0);
        assert!(camera.distance() < initial_distance);
    }

    #[test]
    fn test_zoom_clamp() {
        let mut camera = Camera::new(1.0);
        // Zoom in a lot
        for _ in 0..100 {
            camera.zoom(1.0);
        }
        assert!(camera.distance() >= 0.5); // min_distance

        // Zoom out a lot
        for _ in 0..200 {
            camera.zoom(-1.0);
        }
        assert!(camera.distance() <= 100.0); // max_distance
    }

    #[test]
    fn test_set_target() {
        let mut camera = Camera::new(1.0);
        camera.set_target(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(camera.target(), Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_set_aspect() {
        let mut camera = Camera::new(1.0);
        camera.set_aspect(2.0);
        assert_eq!(camera.aspect, 2.0);
    }

    #[test]
    fn test_focus_on() {
        let mut camera = Camera::new(1.0);
        camera.focus_on(Vec3::new(5.0, 5.0, 5.0));
        assert_eq!(camera.target(), Vec3::new(5.0, 5.0, 5.0));
    }
}
