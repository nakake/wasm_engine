//! ユーティリティモジュール
//!
//! console_log マクロ、ヘルパー関数など

use wasm_bindgen::prelude::*;
use engine_renderer::GizmoAxis;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

/// コンソールにログ出力するマクロ
macro_rules! console_log {
    ($($t:tt)*) => (crate::utils::log(&format_args!($($t)*).to_string()))
}
pub(crate) use console_log;

/// GizmoAxis を文字列に変換
pub fn axis_to_string(axis: GizmoAxis) -> String {
    match axis {
        GizmoAxis::X => "x".to_string(),
        GizmoAxis::Y => "y".to_string(),
        GizmoAxis::Z => "z".to_string(),
        GizmoAxis::XY => "xy".to_string(),
        GizmoAxis::YZ => "yz".to_string(),
        GizmoAxis::XZ => "xz".to_string(),
        GizmoAxis::All => "all".to_string(),
        GizmoAxis::None => "".to_string(),
    }
}
