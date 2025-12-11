// crates/engine-renderer/src/gizmo.rs
//! Gizmo描画システム
//! 選択Entityの位置に移動/回転/スケールGizmoを描画

use glam::{Mat4, Quat, Vec3};
use crate::picking::Ray;

/// Gizmoモード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

/// Gizmo軸
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoAxis {
    #[default]
    None,
    X,
    Y,
    Z,
    XY,
    YZ,
    XZ,
    All,
}

impl std::str::FromStr for GizmoAxis {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "x" => GizmoAxis::X,
            "y" => GizmoAxis::Y,
            "z" => GizmoAxis::Z,
            "xy" => GizmoAxis::XY,
            "yz" => GizmoAxis::YZ,
            "xz" => GizmoAxis::XZ,
            "all" => GizmoAxis::All,
            _ => GizmoAxis::None,
        })
    }
}

/// Gizmo頂点データ
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GizmoVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl GizmoVertex {
    pub fn new(position: Vec3, color: [f32; 4]) -> Self {
        Self {
            position: position.to_array(),
            color,
        }
    }

    /// 頂点バッファレイアウト
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GizmoVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// 軸の色定数
pub const COLOR_X: [f32; 4] = [0.9, 0.2, 0.2, 1.0];       // 赤
pub const COLOR_Y: [f32; 4] = [0.2, 0.9, 0.2, 1.0];       // 緑
pub const COLOR_Z: [f32; 4] = [0.2, 0.2, 0.9, 1.0];       // 青
pub const COLOR_X_HOVER: [f32; 4] = [1.0, 0.5, 0.5, 1.0]; // 赤（ハイライト）
pub const COLOR_Y_HOVER: [f32; 4] = [0.5, 1.0, 0.5, 1.0]; // 緑（ハイライト）
pub const COLOR_Z_HOVER: [f32; 4] = [0.5, 0.5, 1.0, 1.0]; // 青（ハイライト）
pub const COLOR_XY: [f32; 4] = [1.0, 1.0, 0.0, 0.4];      // 黄（半透明）
pub const COLOR_YZ: [f32; 4] = [0.0, 1.0, 1.0, 0.4];      // シアン（半透明）
pub const COLOR_XZ: [f32; 4] = [1.0, 0.0, 1.0, 0.4];      // マゼンタ（半透明）
pub const COLOR_ALL: [f32; 4] = [1.0, 1.0, 1.0, 0.8];     // 白

/// Gizmo状態
#[derive(Debug, Clone)]
pub struct GizmoState {
    /// 現在のモード
    pub mode: GizmoMode,
    /// 表示フラグ
    pub visible: bool,
    /// Gizmo位置
    pub position: Vec3,
    /// Gizmo回転（Local Space用）
    pub rotation: Quat,
    /// ホバー中の軸
    pub hovered_axis: GizmoAxis,
    /// 操作中の軸
    pub active_axis: GizmoAxis,
}

impl Default for GizmoState {
    fn default() -> Self {
        Self {
            mode: GizmoMode::Translate,
            visible: false,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            hovered_axis: GizmoAxis::None,
            active_axis: GizmoAxis::None,
        }
    }
}

// ========================================================================
// ヘルパー関数
// ========================================================================

/// レイと平面の交差判定（距離を返す）
fn ray_plane_intersection(ray: &Ray, plane_point: Vec3, plane_normal: Vec3) -> Option<f32> {
    let denom = ray.direction.dot(plane_normal);
    if denom.abs() < 1e-6 {
        return None; // レイと平面が平行
    }

    let t = (plane_point - ray.origin).dot(plane_normal) / denom;
    Some(t)
}

/// レイと平面の交点を返す
fn ray_plane_intersection_point(ray: &Ray, plane_point: Vec3, plane_normal: Vec3) -> Option<Vec3> {
    let t = ray_plane_intersection(ray, plane_point, plane_normal)?;
    if t < 0.0 {
        return None;
    }
    Some(ray.origin + ray.direction * t)
}

/// レイと円柱の交差判定（無限長円柱ではなく、有限長の線分を太くしたもの）
fn ray_cylinder_intersection(ray: &Ray, start: Vec3, end: Vec3, radius: f32) -> Option<f32> {
    let axis = end - start;
    let axis_len = axis.length();
    if axis_len < 1e-6 {
        return None;
    }
    let axis_dir = axis / axis_len;

    // レイ原点から線分始点へのベクトル
    let oc = ray.origin - start;

    // 円柱軸に垂直な成分でレイと円柱の交差を計算
    let ray_perp = ray.direction - axis_dir * ray.direction.dot(axis_dir);
    let oc_perp = oc - axis_dir * oc.dot(axis_dir);

    let a = ray_perp.dot(ray_perp);
    let b = 2.0 * ray_perp.dot(oc_perp);
    let c = oc_perp.dot(oc_perp) - radius * radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    // 有効な t を探す
    for t in [t1, t2] {
        if t < 0.0 {
            continue;
        }

        let hit_point = ray.origin + ray.direction * t;
        let projection = (hit_point - start).dot(axis_dir);

        // 線分の範囲内かチェック
        if projection >= 0.0 && projection <= axis_len {
            return Some(t);
        }
    }

    None
}

impl GizmoState {
    /// モデル行列を計算（カメラ距離に応じたスケール付き）
    pub fn model_matrix(&self, camera_position: Vec3) -> Mat4 {
        let distance = (camera_position - self.position).length();
        let gizmo_scale = distance * 0.15;

        Mat4::from_scale_rotation_translation(
            Vec3::splat(gizmo_scale),
            self.rotation,
            self.position,
        )
    }

    /// Gizmoスケールを取得（ヒットテスト用）
    pub fn gizmo_scale(&self, camera_position: Vec3) -> f32 {
        let distance = (camera_position - self.position).length();
        distance * 0.15
    }

    /// 軸の色を取得（ホバー/アクティブ状態で変化）
    pub fn axis_color(&self, axis: GizmoAxis) -> [f32; 4] {
        let is_highlighted = self.hovered_axis == axis || self.active_axis == axis;

        match axis {
            GizmoAxis::X => if is_highlighted { COLOR_X_HOVER } else { COLOR_X },
            GizmoAxis::Y => if is_highlighted { COLOR_Y_HOVER } else { COLOR_Y },
            GizmoAxis::Z => if is_highlighted { COLOR_Z_HOVER } else { COLOR_Z },
            GizmoAxis::XY => COLOR_XY,
            GizmoAxis::YZ => COLOR_YZ,
            GizmoAxis::XZ => COLOR_XZ,
            GizmoAxis::All => COLOR_ALL,
            GizmoAxis::None => [0.5, 0.5, 0.5, 0.5],
        }
    }

    /// レイとGizmo軸のヒットテスト
    pub fn hit_test(&self, ray: &Ray, camera_position: Vec3) -> GizmoAxis {
        if !self.visible {
            return GizmoAxis::None;
        }

        let scale = self.gizmo_scale(camera_position);
        let mut closest: Option<(GizmoAxis, f32)> = None;

        // 中央ボックスのヒットテスト (All axis)
        if let Some(t) = self.hit_test_center_box(ray, scale) {
            closest = Some((GizmoAxis::All, t));
        }

        // 各軸のヒットテスト
        match self.mode {
            GizmoMode::Translate | GizmoMode::Scale => {
                for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                    if let Some(t) = self.hit_test_axis(ray, axis, scale) {
                        match closest {
                            None => closest = Some((axis, t)),
                            Some((_, prev_t)) if t < prev_t => closest = Some((axis, t)),
                            _ => {}
                        }
                    }
                }

                // 平面ハンドルのヒットテスト（Translateのみ）
                if self.mode == GizmoMode::Translate {
                    for axis in [GizmoAxis::XY, GizmoAxis::YZ, GizmoAxis::XZ] {
                        if let Some(t) = self.hit_test_plane_handle(ray, axis, scale) {
                            match closest {
                                None => closest = Some((axis, t)),
                                Some((_, prev_t)) if t < prev_t => closest = Some((axis, t)),
                                _ => {}
                            }
                        }
                    }
                }
            }
            GizmoMode::Rotate => {
                for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                    if let Some(t) = self.hit_test_circle(ray, axis, scale) {
                        match closest {
                            None => closest = Some((axis, t)),
                            Some((_, prev_t)) if t < prev_t => closest = Some((axis, t)),
                            _ => {}
                        }
                    }
                }
            }
        }

        closest.map(|(axis, _)| axis).unwrap_or(GizmoAxis::None)
    }

    /// 軸（線分）のヒットテスト
    fn hit_test_axis(&self, ray: &Ray, axis: GizmoAxis, scale: f32) -> Option<f32> {
        let axis_dir = match axis {
            GizmoAxis::X => Vec3::X,
            GizmoAxis::Y => Vec3::Y,
            GizmoAxis::Z => Vec3::Z,
            _ => return None,
        };

        // 軸を円柱として扱う（半径 = threshold）
        let threshold = scale * 0.08;
        let axis_start = self.position;
        let axis_end = self.position + axis_dir * scale;

        ray_cylinder_intersection(ray, axis_start, axis_end, threshold)
    }

    /// 中央ボックスのヒットテスト
    fn hit_test_center_box(&self, ray: &Ray, scale: f32) -> Option<f32> {
        let half = scale * 0.12;
        let min = self.position - Vec3::splat(half);
        let max = self.position + Vec3::splat(half);

        let aabb = crate::picking::AABB::new(min, max);
        ray.intersect_aabb(&aabb)
    }

    /// 平面ハンドルのヒットテスト
    fn hit_test_plane_handle(&self, ray: &Ray, axis: GizmoAxis, scale: f32) -> Option<f32> {
        let (v1, v2, normal) = match axis {
            GizmoAxis::XY => (Vec3::X, Vec3::Y, Vec3::Z),
            GizmoAxis::YZ => (Vec3::Y, Vec3::Z, Vec3::X),
            GizmoAxis::XZ => (Vec3::X, Vec3::Z, Vec3::Y),
            _ => return None,
        };

        let offset = scale * 0.3;
        let size = scale * 0.25;

        // 平面の中心
        let center = self.position + (v1 + v2) * (offset + size * 0.5);

        // レイと平面の交点
        let t = ray_plane_intersection(ray, center, normal)?;
        if t < 0.0 {
            return None;
        }

        let hit_point = ray.origin + ray.direction * t;
        let local = hit_point - self.position;

        // 平面の範囲内かチェック
        let coord1 = local.dot(v1);
        let coord2 = local.dot(v2);

        if coord1 >= offset && coord1 <= offset + size
            && coord2 >= offset && coord2 <= offset + size
        {
            Some(t)
        } else {
            None
        }
    }

    /// 回転用円のヒットテスト
    fn hit_test_circle(&self, ray: &Ray, axis: GizmoAxis, scale: f32) -> Option<f32> {
        let normal = match axis {
            GizmoAxis::X => Vec3::X,
            GizmoAxis::Y => Vec3::Y,
            GizmoAxis::Z => Vec3::Z,
            _ => return None,
        };

        // 円の平面との交点を計算
        let t = ray_plane_intersection(ray, self.position, normal)?;
        if t < 0.0 {
            return None;
        }

        let hit_point = ray.origin + ray.direction * t;
        let dist = (hit_point - self.position).length();

        // 円のリング内かチェック（半径 ± threshold）
        let radius = scale;
        let threshold = scale * 0.1;

        if (dist - radius).abs() < threshold {
            Some(t)
        } else {
            None
        }
    }

    /// Translateモードのドラッグ移動量を計算
    pub fn calculate_translate_drag(
        &self,
        axis: GizmoAxis,
        ray: &Ray,
        prev_ray: &Ray,
        camera_position: Vec3,
    ) -> Vec3 {
        let plane_normal = self.drag_plane_normal(axis, camera_position);

        let prev_point = ray_plane_intersection_point(prev_ray, self.position, plane_normal);
        let curr_point = ray_plane_intersection_point(ray, self.position, plane_normal);

        match (prev_point, curr_point) {
            (Some(prev), Some(curr)) => {
                let delta = curr - prev;
                // 軸方向成分のみ取り出す
                match axis {
                    GizmoAxis::X => Vec3::new(delta.x, 0.0, 0.0),
                    GizmoAxis::Y => Vec3::new(0.0, delta.y, 0.0),
                    GizmoAxis::Z => Vec3::new(0.0, 0.0, delta.z),
                    GizmoAxis::XY => Vec3::new(delta.x, delta.y, 0.0),
                    GizmoAxis::YZ => Vec3::new(0.0, delta.y, delta.z),
                    GizmoAxis::XZ => Vec3::new(delta.x, 0.0, delta.z),
                    GizmoAxis::All => delta,
                    GizmoAxis::None => Vec3::ZERO,
                }
            }
            _ => Vec3::ZERO,
        }
    }

    /// Scaleモードのドラッグスケール量を計算
    pub fn calculate_scale_drag(
        &self,
        axis: GizmoAxis,
        ray: &Ray,
        prev_ray: &Ray,
        camera_position: Vec3,
    ) -> Vec3 {
        // Translateと同じ計算で移動量を取得し、スケール変化量として解釈
        let delta = self.calculate_translate_drag(axis, ray, prev_ray, camera_position);
        let scale = self.gizmo_scale(camera_position);

        // スケール変化を正規化（Gizmoサイズに対する比率）
        delta / scale
    }

    /// Rotateモードのドラッグ回転量を計算（Quaternion）
    pub fn calculate_rotate_drag(
        &self,
        axis: GizmoAxis,
        ray: &Ray,
        prev_ray: &Ray,
    ) -> Quat {
        let rotation_axis = match axis {
            GizmoAxis::X => Vec3::X,
            GizmoAxis::Y => Vec3::Y,
            GizmoAxis::Z => Vec3::Z,
            _ => return Quat::IDENTITY,
        };

        // 回転平面との交点
        let prev_point = ray_plane_intersection_point(prev_ray, self.position, rotation_axis);
        let curr_point = ray_plane_intersection_point(ray, self.position, rotation_axis);

        match (prev_point, curr_point) {
            (Some(prev), Some(curr)) => {
                let prev_dir = (prev - self.position).normalize();
                let curr_dir = (curr - self.position).normalize();

                if prev_dir.length_squared() < 0.0001 || curr_dir.length_squared() < 0.0001 {
                    return Quat::IDENTITY;
                }

                // 2つのベクトル間の角度
                let dot = prev_dir.dot(curr_dir).clamp(-1.0, 1.0);
                let angle = dot.acos();

                // 回転方向を決定
                let cross = prev_dir.cross(curr_dir);
                let sign = if cross.dot(rotation_axis) >= 0.0 { 1.0 } else { -1.0 };

                Quat::from_axis_angle(rotation_axis, angle * sign)
            }
            _ => Quat::IDENTITY,
        }
    }

    /// ドラッグ用の平面法線を計算
    fn drag_plane_normal(&self, axis: GizmoAxis, camera_position: Vec3) -> Vec3 {
        let view_dir = (self.position - camera_position).normalize();

        match axis {
            GizmoAxis::X => {
                // X軸操作：YまたはZ平面のうち、視線に対して垂直に近い方を使う
                if view_dir.y.abs() > view_dir.z.abs() {
                    Vec3::Y
                } else {
                    Vec3::Z
                }
            }
            GizmoAxis::Y => {
                if view_dir.x.abs() > view_dir.z.abs() {
                    Vec3::X
                } else {
                    Vec3::Z
                }
            }
            GizmoAxis::Z => {
                if view_dir.x.abs() > view_dir.y.abs() {
                    Vec3::X
                } else {
                    Vec3::Y
                }
            }
            GizmoAxis::XY => Vec3::Z,
            GizmoAxis::YZ => Vec3::X,
            GizmoAxis::XZ => Vec3::Y,
            GizmoAxis::All => view_dir,
            GizmoAxis::None => Vec3::Y,
        }
    }
}

// ========================================================================
// メッシュ生成関数
// ========================================================================

/// Translate Gizmo用の矢印メッシュを生成
pub fn create_arrow_vertices(axis: GizmoAxis, color: [f32; 4]) -> Vec<GizmoVertex> {
    let mut vertices = Vec::new();

    // 軸方向を決定
    let (dir, up) = match axis {
        GizmoAxis::X => (Vec3::X, Vec3::Y),
        GizmoAxis::Y => (Vec3::Y, Vec3::Z),
        GizmoAxis::Z => (Vec3::Z, Vec3::Y),
        _ => return vertices,
    };

    let right = dir.cross(up).normalize();

    // 線の太さ
    let thickness = 0.02;
    let length = 1.0;
    let cone_length = 0.2;
    let cone_radius = 0.06;

    // シャフト（四角柱）
    let shaft_end = dir * (length - cone_length);

    // シャフトの4頂点（始点）
    let s0 = up * thickness + right * thickness;
    let s1 = up * thickness - right * thickness;
    let s2 = -up * thickness - right * thickness;
    let s3 = -up * thickness + right * thickness;

    // シャフトの4頂点（終点）
    let e0 = shaft_end + up * thickness + right * thickness;
    let e1 = shaft_end + up * thickness - right * thickness;
    let e2 = shaft_end - up * thickness - right * thickness;
    let e3 = shaft_end - up * thickness + right * thickness;

    // シャフト面（6面 x 2三角形）
    // 前面
    vertices.push(GizmoVertex::new(s0, color));
    vertices.push(GizmoVertex::new(s1, color));
    vertices.push(GizmoVertex::new(e1, color));
    vertices.push(GizmoVertex::new(s0, color));
    vertices.push(GizmoVertex::new(e1, color));
    vertices.push(GizmoVertex::new(e0, color));

    // 後面
    vertices.push(GizmoVertex::new(s2, color));
    vertices.push(GizmoVertex::new(s3, color));
    vertices.push(GizmoVertex::new(e3, color));
    vertices.push(GizmoVertex::new(s2, color));
    vertices.push(GizmoVertex::new(e3, color));
    vertices.push(GizmoVertex::new(e2, color));

    // 上面
    vertices.push(GizmoVertex::new(s0, color));
    vertices.push(GizmoVertex::new(e0, color));
    vertices.push(GizmoVertex::new(e3, color));
    vertices.push(GizmoVertex::new(s0, color));
    vertices.push(GizmoVertex::new(e3, color));
    vertices.push(GizmoVertex::new(s3, color));

    // 下面
    vertices.push(GizmoVertex::new(s1, color));
    vertices.push(GizmoVertex::new(s2, color));
    vertices.push(GizmoVertex::new(e2, color));
    vertices.push(GizmoVertex::new(s1, color));
    vertices.push(GizmoVertex::new(e2, color));
    vertices.push(GizmoVertex::new(e1, color));

    // コーン（矢印先端）
    let tip = dir * length;
    let cone_base = shaft_end;
    let segments = 8;

    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

        let p1 = cone_base + (up * angle1.cos() + right * angle1.sin()) * cone_radius;
        let p2 = cone_base + (up * angle2.cos() + right * angle2.sin()) * cone_radius;

        // 側面
        vertices.push(GizmoVertex::new(p1, color));
        vertices.push(GizmoVertex::new(p2, color));
        vertices.push(GizmoVertex::new(tip, color));

        // 底面
        vertices.push(GizmoVertex::new(cone_base, color));
        vertices.push(GizmoVertex::new(p1, color));
        vertices.push(GizmoVertex::new(p2, color));
    }

    vertices
}

/// 平面ハンドル用のメッシュを生成
pub fn create_plane_vertices(axis: GizmoAxis, color: [f32; 4]) -> Vec<GizmoVertex> {
    let mut vertices = Vec::new();

    let size = 0.25;
    let offset = 0.3;

    let (v1, v2) = match axis {
        GizmoAxis::XY => (Vec3::X, Vec3::Y),
        GizmoAxis::YZ => (Vec3::Y, Vec3::Z),
        GizmoAxis::XZ => (Vec3::X, Vec3::Z),
        _ => return vertices,
    };

    let p0 = v1 * offset + v2 * offset;
    let p1 = v1 * (offset + size) + v2 * offset;
    let p2 = v1 * (offset + size) + v2 * (offset + size);
    let p3 = v1 * offset + v2 * (offset + size);

    // 両面描画
    vertices.push(GizmoVertex::new(p0, color));
    vertices.push(GizmoVertex::new(p1, color));
    vertices.push(GizmoVertex::new(p2, color));
    vertices.push(GizmoVertex::new(p0, color));
    vertices.push(GizmoVertex::new(p2, color));
    vertices.push(GizmoVertex::new(p3, color));

    vertices.push(GizmoVertex::new(p0, color));
    vertices.push(GizmoVertex::new(p2, color));
    vertices.push(GizmoVertex::new(p1, color));
    vertices.push(GizmoVertex::new(p0, color));
    vertices.push(GizmoVertex::new(p3, color));
    vertices.push(GizmoVertex::new(p2, color));

    vertices
}

/// Rotate Gizmo用の円メッシュを生成
pub fn create_circle_vertices(axis: GizmoAxis, color: [f32; 4]) -> Vec<GizmoVertex> {
    let mut vertices = Vec::new();

    let radius = 1.0;
    let thickness = 0.03;
    let segments = 32;

    let (normal, up) = match axis {
        GizmoAxis::X => (Vec3::X, Vec3::Y),
        GizmoAxis::Y => (Vec3::Y, Vec3::Z),
        GizmoAxis::Z => (Vec3::Z, Vec3::Y),
        _ => return vertices,
    };

    let right = normal.cross(up).normalize();
    let up = right.cross(normal).normalize();

    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

        let p1 = (up * angle1.cos() + right * angle1.sin()) * radius;
        let p2 = (up * angle2.cos() + right * angle2.sin()) * radius;

        // 内側と外側
        let inner1 = p1 * (1.0 - thickness / radius);
        let outer1 = p1 * (1.0 + thickness / radius);
        let inner2 = p2 * (1.0 - thickness / radius);
        let outer2 = p2 * (1.0 + thickness / radius);

        // 帯状に描画
        vertices.push(GizmoVertex::new(inner1, color));
        vertices.push(GizmoVertex::new(outer1, color));
        vertices.push(GizmoVertex::new(outer2, color));

        vertices.push(GizmoVertex::new(inner1, color));
        vertices.push(GizmoVertex::new(outer2, color));
        vertices.push(GizmoVertex::new(inner2, color));
    }

    vertices
}

/// Scale Gizmo用のボックス付き線メッシュを生成
pub fn create_scale_axis_vertices(axis: GizmoAxis, color: [f32; 4]) -> Vec<GizmoVertex> {
    let mut vertices = Vec::new();

    let dir = match axis {
        GizmoAxis::X => Vec3::X,
        GizmoAxis::Y => Vec3::Y,
        GizmoAxis::Z => Vec3::Z,
        _ => return vertices,
    };

    let up = if axis == GizmoAxis::Y { Vec3::Z } else { Vec3::Y };
    let right = dir.cross(up).normalize();
    let up = right.cross(dir).normalize();

    let thickness = 0.02;
    let length = 0.85;
    let box_size = 0.1;

    // シャフト
    let shaft_end = dir * length;

    let s0 = up * thickness + right * thickness;
    let s1 = up * thickness - right * thickness;
    let s2 = -up * thickness - right * thickness;
    let s3 = -up * thickness + right * thickness;

    let e0 = shaft_end + up * thickness + right * thickness;
    let e1 = shaft_end + up * thickness - right * thickness;
    let e2 = shaft_end - up * thickness - right * thickness;
    let e3 = shaft_end - up * thickness + right * thickness;

    // シャフト4面
    vertices.push(GizmoVertex::new(s0, color));
    vertices.push(GizmoVertex::new(s1, color));
    vertices.push(GizmoVertex::new(e1, color));
    vertices.push(GizmoVertex::new(s0, color));
    vertices.push(GizmoVertex::new(e1, color));
    vertices.push(GizmoVertex::new(e0, color));

    vertices.push(GizmoVertex::new(s2, color));
    vertices.push(GizmoVertex::new(s3, color));
    vertices.push(GizmoVertex::new(e3, color));
    vertices.push(GizmoVertex::new(s2, color));
    vertices.push(GizmoVertex::new(e3, color));
    vertices.push(GizmoVertex::new(e2, color));

    vertices.push(GizmoVertex::new(s0, color));
    vertices.push(GizmoVertex::new(e0, color));
    vertices.push(GizmoVertex::new(e3, color));
    vertices.push(GizmoVertex::new(s0, color));
    vertices.push(GizmoVertex::new(e3, color));
    vertices.push(GizmoVertex::new(s3, color));

    vertices.push(GizmoVertex::new(s1, color));
    vertices.push(GizmoVertex::new(s2, color));
    vertices.push(GizmoVertex::new(e2, color));
    vertices.push(GizmoVertex::new(s1, color));
    vertices.push(GizmoVertex::new(e2, color));
    vertices.push(GizmoVertex::new(e1, color));

    // 先端ボックス
    let box_center = dir * (length + box_size * 0.5);
    let half = box_size * 0.5;

    let corners = [
        box_center + Vec3::new(-half, -half, -half),
        box_center + Vec3::new(half, -half, -half),
        box_center + Vec3::new(half, half, -half),
        box_center + Vec3::new(-half, half, -half),
        box_center + Vec3::new(-half, -half, half),
        box_center + Vec3::new(half, -half, half),
        box_center + Vec3::new(half, half, half),
        box_center + Vec3::new(-half, half, half),
    ];

    // 6面
    let faces = [
        [0, 1, 2, 3], // front
        [5, 4, 7, 6], // back
        [4, 0, 3, 7], // left
        [1, 5, 6, 2], // right
        [3, 2, 6, 7], // top
        [4, 5, 1, 0], // bottom
    ];

    for face in faces {
        vertices.push(GizmoVertex::new(corners[face[0]], color));
        vertices.push(GizmoVertex::new(corners[face[1]], color));
        vertices.push(GizmoVertex::new(corners[face[2]], color));
        vertices.push(GizmoVertex::new(corners[face[0]], color));
        vertices.push(GizmoVertex::new(corners[face[2]], color));
        vertices.push(GizmoVertex::new(corners[face[3]], color));
    }

    vertices
}

/// 中心の全軸スケール用ボックスを生成
pub fn create_center_box_vertices(color: [f32; 4]) -> Vec<GizmoVertex> {
    let mut vertices = Vec::new();
    let half = 0.1;

    let corners = [
        Vec3::new(-half, -half, -half),
        Vec3::new(half, -half, -half),
        Vec3::new(half, half, -half),
        Vec3::new(-half, half, -half),
        Vec3::new(-half, -half, half),
        Vec3::new(half, -half, half),
        Vec3::new(half, half, half),
        Vec3::new(-half, half, half),
    ];

    let faces = [
        [0, 1, 2, 3],
        [5, 4, 7, 6],
        [4, 0, 3, 7],
        [1, 5, 6, 2],
        [3, 2, 6, 7],
        [4, 5, 1, 0],
    ];

    for face in faces {
        vertices.push(GizmoVertex::new(corners[face[0]], color));
        vertices.push(GizmoVertex::new(corners[face[1]], color));
        vertices.push(GizmoVertex::new(corners[face[2]], color));
        vertices.push(GizmoVertex::new(corners[face[0]], color));
        vertices.push(GizmoVertex::new(corners[face[2]], color));
        vertices.push(GizmoVertex::new(corners[face[3]], color));
    }

    vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_axis_from_str() {
        assert_eq!("x".parse::<GizmoAxis>().unwrap(), GizmoAxis::X);
        assert_eq!("Y".parse::<GizmoAxis>().unwrap(), GizmoAxis::Y);
        assert_eq!("xy".parse::<GizmoAxis>().unwrap(), GizmoAxis::XY);
        assert_eq!("invalid".parse::<GizmoAxis>().unwrap(), GizmoAxis::None);
    }

    #[test]
    fn test_arrow_vertices() {
        let vertices = create_arrow_vertices(GizmoAxis::X, COLOR_X);
        assert!(!vertices.is_empty());
    }

    #[test]
    fn test_circle_vertices() {
        let vertices = create_circle_vertices(GizmoAxis::Y, COLOR_Y);
        assert!(!vertices.is_empty());
    }

    #[test]
    fn test_scale_vertices() {
        let vertices = create_scale_axis_vertices(GizmoAxis::Z, COLOR_Z);
        assert!(!vertices.is_empty());
    }
}
