// crates/engine-wasm/src/lib.rs

mod renderer;
mod shaders;
mod subscription;
mod utils;

use renderer::Renderer;
use subscription::{calculate_hash, QuerySubscriptionManager};
use utils::console_log;

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use js_sys::Function;

use engine_core::{EntityId, Name, QueryDescriptor, Transform, World};
use engine_renderer::{GizmoAxis, GizmoMode, Ray};
use glam::{Quat, Vec3};

// パニック時のスタックトレース表示
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Engine構造体
/// WorldとRendererを統合し、JSから操作可能なAPIを提供
#[wasm_bindgen]
pub struct Engine {
    world: World,
    renderer: Renderer,
    subscriptions: QuerySubscriptionManager,
    /// Gizmoドラッグ開始時のレイ
    gizmo_drag_ray: Option<Ray>,
    /// Gizmoドラッグ中の軸
    gizmo_drag_axis: GizmoAxis,
}

#[wasm_bindgen]
impl Engine {
    /// 新しいEngineを作成（非同期）
    pub async fn create(canvas: HtmlCanvasElement) -> Result<Engine, JsValue> {
        console_log!("Creating Engine...");
        let renderer = Renderer::create(canvas).await?;
        let world = World::new();
        let subscriptions = QuerySubscriptionManager::new();
        console_log!("Engine created successfully");
        Ok(Self {
            world,
            renderer,
            subscriptions,
            gizmo_drag_ray: None,
            gizmo_drag_axis: GizmoAxis::None,
        })
    }

    /// Entityを作成し、IDを返す
    pub fn create_entity(&mut self, name: &str) -> u32 {
        let entity = self.world.spawn();
        self.world.insert(entity, Name::new(name));
        self.world.insert(entity, Transform::identity());
        console_log!("Created entity: {} (id: {})", name, entity.to_u32());
        self.check_subscriptions();
        entity.to_u32()
    }

    /// Entityを削除
    pub fn delete_entity(&mut self, id: u32) -> bool {
        let entity = EntityId::from_u32(id);
        let result = self.world.despawn(entity);
        if result {
            console_log!("Deleted entity: {}", id);
            self.check_subscriptions();
        }
        result
    }

    /// 位置を設定
    pub fn set_position(&mut self, id: u32, x: f32, y: f32, z: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.position = Vec3::new(x, y, z);
            self.check_subscriptions();
        }
    }

    /// 回転を設定（クォータニオン）
    pub fn set_rotation(&mut self, id: u32, x: f32, y: f32, z: f32, w: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.rotation = Quat::from_xyzw(x, y, z, w);
            self.check_subscriptions();
        }
    }

    /// スケールを設定
    pub fn set_scale(&mut self, id: u32, x: f32, y: f32, z: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.scale = Vec3::new(x, y, z);
            self.check_subscriptions();
        }
    }

    /// 位置を取得（x, y, zの配列）
    pub fn get_position(&self, id: u32) -> Option<Vec<f32>> {
        let entity = EntityId::from_u32(id);
        self.world
            .get::<Transform>(entity)
            .map(|t| vec![t.position.x, t.position.y, t.position.z])
    }

    /// 回転を取得（x, y, z, wの配列）
    pub fn get_rotation(&self, id: u32) -> Option<Vec<f32>> {
        let entity = EntityId::from_u32(id);
        self.world.get::<Transform>(entity).map(|t| {
            vec![
                t.rotation.x,
                t.rotation.y,
                t.rotation.z,
                t.rotation.w,
            ]
        })
    }

    /// スケールを取得（x, y, zの配列）
    pub fn get_scale(&self, id: u32) -> Option<Vec<f32>> {
        let entity = EntityId::from_u32(id);
        self.world
            .get::<Transform>(entity)
            .map(|t| vec![t.scale.x, t.scale.y, t.scale.z])
    }

    /// Entity名を取得
    pub fn get_name(&self, id: u32) -> Option<String> {
        let entity = EntityId::from_u32(id);
        self.world
            .get::<Name>(entity)
            .map(|n| n.as_str().to_string())
    }

    /// Entity名を設定
    pub fn set_name(&mut self, id: u32, name: &str) {
        let entity = EntityId::from_u32(id);
        if let Some(n) = self.world.get_mut::<Name>(entity) {
            *n = Name::new(name);
            self.check_subscriptions();
        }
    }

    /// Entityが生存しているか確認
    pub fn is_alive(&self, id: u32) -> bool {
        let entity = EntityId::from_u32(id);
        self.world.is_alive(entity)
    }

    /// Entity数を取得
    pub fn entity_count(&self) -> usize {
        self.world.entity_count()
    }

    /// フレーム更新（レンダリング含む）
    pub fn tick(&mut self, _delta_time: f32) -> Result<(), JsValue> {
        self.renderer.render_world(&self.world)
    }

    /// Canvasリサイズ
    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    /// 幅取得
    pub fn width(&self) -> u32 {
        self.renderer.width()
    }

    /// 高さ取得
    pub fn height(&self) -> u32 {
        self.renderer.height()
    }

    /// クエリ実行
    ///
    /// # Arguments
    /// * `query_json` - QueryDescriptor の JSON文字列
    ///
    /// # Returns
    /// QueryResult の JsValue（JSON形式）
    pub fn execute_query(&self, query_json: &str) -> Result<JsValue, JsValue> {
        // JSONパース
        let query: QueryDescriptor = serde_json::from_str(query_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid query JSON: {}", e)))?;

        // クエリ実行
        let result = self.world.execute_query(&query);

        // JsValueに変換
        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// クエリを購読
    ///
    /// # Arguments
    /// * `query_json` - QueryDescriptor の JSON文字列
    /// * `callback` - 結果変更時に呼ばれる関数
    ///
    /// # Returns
    /// subscription_id
    pub fn subscribe_query(
        &mut self,
        query_json: &str,
        callback: Function,
    ) -> Result<u32, JsValue> {
        let query: QueryDescriptor = serde_json::from_str(query_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid query JSON: {}", e)))?;

        let id = self.subscriptions.subscribe(query, callback);

        // 初回実行
        self.notify_subscription(id);

        Ok(id)
    }

    /// 購読解除
    pub fn unsubscribe_query(&mut self, subscription_id: u32) -> bool {
        self.subscriptions.unsubscribe(subscription_id)
    }

    // ========================================================================
    // カメラ操作 API
    // ========================================================================

    /// カメラをターゲット周りで回転（Orbit）
    ///
    /// # Arguments
    /// * `delta_x` - 水平方向の回転量（ラジアン）
    /// * `delta_y` - 垂直方向の回転量（ラジアン）
    pub fn orbit_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.renderer.orbit_camera(delta_x, delta_y);
    }

    /// カメラを平行移動（Pan）
    ///
    /// # Arguments
    /// * `delta_x` - 右方向への移動量
    /// * `delta_y` - 上方向への移動量
    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.renderer.pan_camera(delta_x, delta_y);
    }

    /// カメラをズーム
    ///
    /// # Arguments
    /// * `delta` - 正で近づく、負で遠ざかる
    pub fn zoom_camera(&mut self, delta: f32) {
        self.renderer.zoom_camera(delta);
    }

    /// カメラターゲットを設定
    pub fn set_camera_target(&mut self, x: f32, y: f32, z: f32) {
        self.renderer.set_camera_target(Vec3::new(x, y, z));
    }

    /// カメラ位置を取得
    pub fn get_camera_position(&self) -> Vec<f32> {
        let pos = self.renderer.camera_position();
        vec![pos.x, pos.y, pos.z]
    }

    /// カメラターゲットを取得
    pub fn get_camera_target(&self) -> Vec<f32> {
        let target = self.renderer.camera_target();
        vec![target.x, target.y, target.z]
    }

    // ========================================================================
    // Entity Picking API
    // ========================================================================

    /// スクリーン座標からEntityを選択（レイキャスト）
    ///
    /// # Arguments
    /// * `screen_x` - スクリーンX座標 (0.0〜1.0)
    /// * `screen_y` - スクリーンY座標 (0.0〜1.0)
    ///
    /// # Returns
    /// * Entity ID (>= 0) if hit
    /// * -1 if no entity was hit
    pub fn pick_entity(&self, screen_x: f32, screen_y: f32) -> i32 {
        let ray = self.renderer.screen_to_ray(screen_x, screen_y);

        let mut closest: Option<(EntityId, f32)> = None;

        // 全Entityをチェック
        for entity_id in self.world.iter_entities() {
            if let Some(transform) = self.world.get::<Transform>(entity_id) {
                // 簡易Bounding Box (1x1x1 cube at transform position)
                let aabb = engine_renderer::AABB::unit_cube(transform.position, transform.scale);

                if let Some(t) = ray.intersect_aabb(&aabb) {
                    match closest {
                        None => closest = Some((entity_id, t)),
                        Some((_, prev_t)) if t < prev_t => closest = Some((entity_id, t)),
                        _ => {}
                    }
                }
            }
        }

        match closest {
            Some((id, _)) => id.to_u32() as i32,
            None => -1,
        }
    }

    /// スクリーン座標をワールド座標に変換
    ///
    /// # Arguments
    /// * `screen_x` - スクリーンX座標 (0.0〜1.0)
    /// * `screen_y` - スクリーンY座標 (0.0〜1.0)
    /// * `depth` - 深度値 (0.0 = near, 1.0 = far)
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32, depth: f32) -> Vec<f32> {
        let world_pos = self.renderer.screen_to_world(screen_x, screen_y, depth);
        vec![world_pos.x, world_pos.y, world_pos.z]
    }

    // ========================================================================
    // Gizmo API
    // ========================================================================

    /// Gizmoモードを設定
    /// @param mode - "translate" | "rotate" | "scale"
    pub fn set_gizmo_mode(&mut self, mode: &str) {
        self.renderer.set_gizmo_mode(mode);
    }

    /// Gizmo表示/非表示を設定
    pub fn set_gizmo_visible(&mut self, visible: bool) {
        self.renderer.set_gizmo_visible(visible);
    }

    /// Gizmo位置を設定
    pub fn set_gizmo_position(&mut self, x: f32, y: f32, z: f32) {
        self.renderer.set_gizmo_position(x, y, z);
    }

    /// Gizmo回転を設定（Local Space用）
    pub fn set_gizmo_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) {
        self.renderer.set_gizmo_rotation(x, y, z, w);
    }

    /// ホバー中の軸を設定
    /// @param axis - "x" | "y" | "z" | "xy" | "yz" | "xz" | "all" | "none"
    pub fn set_gizmo_hovered_axis(&mut self, axis: &str) {
        self.renderer.set_gizmo_hovered_axis(axis);
    }

    /// アクティブ（操作中）の軸を設定
    pub fn set_gizmo_active_axis(&mut self, axis: &str) {
        self.renderer.set_gizmo_active_axis(axis);
    }

    /// Gizmo表示状態を取得
    pub fn is_gizmo_visible(&self) -> bool {
        self.renderer.is_gizmo_visible()
    }

    /// Gizmoヒットテスト
    /// @param screen_x スクリーンX座標 (0.0〜1.0)
    /// @param screen_y スクリーンY座標 (0.0〜1.0)
    /// @returns ヒットした軸名 ("x", "y", "z", "xy", "yz", "xz", "all", "")
    pub fn gizmo_hit_test(&self, screen_x: f32, screen_y: f32) -> String {
        let ray = self.renderer.camera.screen_to_ray(screen_x, screen_y);
        let camera_pos = self.renderer.camera.position();
        let axis = self.renderer.gizmo_state.hit_test(&ray, camera_pos);
        utils::axis_to_string(axis)
    }

    /// Gizmoドラッグ開始
    /// @param screen_x スクリーンX座標 (0.0〜1.0)
    /// @param screen_y スクリーンY座標 (0.0〜1.0)
    /// @returns ドラッグ開始した軸名（""の場合はヒットなし）
    pub fn start_gizmo_drag(&mut self, screen_x: f32, screen_y: f32) -> String {
        let ray = self.renderer.camera.screen_to_ray(screen_x, screen_y);
        let camera_pos = self.renderer.camera.position();
        let axis = self.renderer.gizmo_state.hit_test(&ray, camera_pos);

        if axis != GizmoAxis::None {
            self.gizmo_drag_ray = Some(ray);
            self.gizmo_drag_axis = axis;
            self.renderer.gizmo_state.active_axis = axis;
        }

        utils::axis_to_string(axis)
    }

    /// Gizmoドラッグ更新（Translate/Scaleモード）
    /// @param screen_x スクリーンX座標 (0.0〜1.0)
    /// @param screen_y スクリーンY座標 (0.0〜1.0)
    /// @returns [dx, dy, dz] 移動/スケール変化量
    pub fn update_gizmo_drag(&mut self, screen_x: f32, screen_y: f32) -> Vec<f32> {
        if self.gizmo_drag_axis == GizmoAxis::None {
            return vec![0.0, 0.0, 0.0];
        }

        let ray = self.renderer.camera.screen_to_ray(screen_x, screen_y);
        let camera_pos = self.renderer.camera.position();

        let prev_ray = self.gizmo_drag_ray.unwrap_or(ray);

        let delta = match self.renderer.gizmo_state.mode {
            GizmoMode::Translate => {
                self.renderer.gizmo_state.calculate_translate_drag(
                    self.gizmo_drag_axis, &ray, &prev_ray, camera_pos
                )
            }
            GizmoMode::Scale => {
                self.renderer.gizmo_state.calculate_scale_drag(
                    self.gizmo_drag_axis, &ray, &prev_ray, camera_pos
                )
            }
            GizmoMode::Rotate => {
                // Rotateモードは update_gizmo_drag_rotate を使う
                Vec3::ZERO
            }
        };

        self.gizmo_drag_ray = Some(ray);

        vec![delta.x, delta.y, delta.z]
    }

    /// Gizmoドラッグ更新（Rotateモード）
    /// @param screen_x スクリーンX座標 (0.0〜1.0)
    /// @param screen_y スクリーンY座標 (0.0〜1.0)
    /// @returns [qx, qy, qz, qw] 回転差分（Quaternion）
    pub fn update_gizmo_drag_rotate(&mut self, screen_x: f32, screen_y: f32) -> Vec<f32> {
        if self.gizmo_drag_axis == GizmoAxis::None {
            return vec![0.0, 0.0, 0.0, 1.0];
        }

        let ray = self.renderer.camera.screen_to_ray(screen_x, screen_y);
        let prev_ray = self.gizmo_drag_ray.unwrap_or(ray);

        let rot = self.renderer.gizmo_state.calculate_rotate_drag(
            self.gizmo_drag_axis, &ray, &prev_ray
        );

        self.gizmo_drag_ray = Some(ray);

        vec![rot.x, rot.y, rot.z, rot.w]
    }

    /// Gizmoドラッグ終了
    pub fn end_gizmo_drag(&mut self) {
        self.gizmo_drag_ray = None;
        self.gizmo_drag_axis = GizmoAxis::None;
        self.renderer.gizmo_state.active_axis = GizmoAxis::None;
    }

    /// 全購読のチェック・通知
    fn check_subscriptions(&mut self) {
        let ids = self.subscriptions.subscription_ids();
        for id in ids {
            self.notify_subscription(id);
        }
    }

    /// 単一の購読を通知
    fn notify_subscription(&mut self, id: u32) {
        if let Some(sub) = self.subscriptions.get_mut(id) {
            let result = self.world.execute_query(&sub.query);
            let hash = calculate_hash(&result);

            if hash != sub.last_result_hash {
                sub.last_result_hash = hash;

                // コールバック呼び出し
                if let Ok(js_result) = serde_wasm_bindgen::to_value(&result) {
                    let _ = sub.callback.call1(&JsValue::NULL, &js_result);
                }
            }
        }
    }
}
