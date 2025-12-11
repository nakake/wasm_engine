//! シェーダーモジュール
//!
//! WGSLシェーダーを外部ファイルから読み込む

/// メインシェーダー（3Dオブジェクト描画用）
pub const MAIN_SHADER: &str = include_str!("main.wgsl");

/// Gizmoシェーダー（Gizmo描画用）
pub const GIZMO_SHADER: &str = include_str!("gizmo.wgsl");
