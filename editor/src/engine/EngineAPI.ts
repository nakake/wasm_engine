import type { EntityId, Vec3, Quat, EntityData, Transform, QueryDescriptor, QueryResult, GizmoMode, GizmoAxis } from './types';
import { Vec3 as Vec3Helper, Quat as QuatHelper } from './types';
import { EntityQueryBuilder } from './query';

// WASMエンジン型（wasm-packで生成される）
interface WasmEngine {
  create_entity(name: string): number;
  delete_entity(id: number): boolean;
  set_position(id: number, x: number, y: number, z: number): void;
  set_rotation(id: number, x: number, y: number, z: number, w: number): void;
  set_scale(id: number, x: number, y: number, z: number): void;
  set_name(id: number, name: string): void;
  get_position(id: number): number[] | undefined;
  get_rotation(id: number): number[] | undefined;
  get_scale(id: number): number[] | undefined;
  get_name(id: number): string | undefined;
  is_alive(id: number): boolean;
  entity_count(): number;
  tick(delta_time: number): void;
  resize(width: number, height: number): void;
  width(): number;
  height(): number;
  execute_query(query_json: string): QueryResult;
  subscribe_query(query_json: string, callback: (result: QueryResult) => void): number;
  unsubscribe_query(subscription_id: number): boolean;
  // Camera API
  orbit_camera(delta_x: number, delta_y: number): void;
  pan_camera(delta_x: number, delta_y: number): void;
  zoom_camera(delta: number): void;
  set_camera_target(x: number, y: number, z: number): void;
  get_camera_position(): number[];
  get_camera_target(): number[];
  // Picking API
  pick_entity(screen_x: number, screen_y: number): number;
  screen_to_world(screen_x: number, screen_y: number, depth: number): number[];
  // Gizmo API
  set_gizmo_mode(mode: string): void;
  set_gizmo_visible(visible: boolean): void;
  set_gizmo_position(x: number, y: number, z: number): void;
  set_gizmo_rotation(x: number, y: number, z: number, w: number): void;
  set_gizmo_hovered_axis(axis: string): void;
  set_gizmo_active_axis(axis: string): void;
  is_gizmo_visible(): boolean;
  // Gizmo Interaction API
  gizmo_hit_test(screen_x: number, screen_y: number): string;
  start_gizmo_drag(screen_x: number, screen_y: number): string;
  update_gizmo_drag(screen_x: number, screen_y: number): number[];
  update_gizmo_drag_rotate(screen_x: number, screen_y: number): number[];
  end_gizmo_drag(): void;
  free(): void;
}

// WASMモジュール型
interface WasmModule {
  Engine: {
    create(canvas: HTMLCanvasElement): Promise<WasmEngine>;
  };
}

/**
 * WASM Engine APIのTypeScriptラッパー
 */
export class EngineAPI {
  private engine: WasmEngine | null = null;
  private entities: Map<EntityId, string> = new Map();
  private initialized = false;

  /**
   * エンジンが初期化済みか確認
   */
  get isInitialized(): boolean {
    return this.initialized && this.engine !== null;
  }

  /**
   * Canvas要素でエンジンを初期化
   */
  async initialize(canvas: HTMLCanvasElement): Promise<void> {
    if (this.initialized) {
      console.warn('Engine already initialized');
      return;
    }

    try {
      // WASMモジュールを動的インポート
      const wasm = await import('../wasm/engine_wasm') as unknown as WasmModule;
      this.engine = await wasm.Engine.create(canvas);
      this.initialized = true;
      console.log('EngineAPI initialized');
    } catch (error) {
      console.error('Failed to initialize engine:', error);
      throw error;
    }
  }

  /**
   * 指定した名前でEntityを作成
   */
  createEntity(name: string): EntityId {
    const engine = this.getEngine();
    const id = engine.create_entity(name);
    this.entities.set(id, name);
    return id;
  }

  /**
   * IDでEntityを削除
   */
  deleteEntity(id: EntityId): boolean {
    const engine = this.getEngine();
    const result = engine.delete_entity(id);
    if (result) {
      this.entities.delete(id);
    }
    return result;
  }

  /**
   * Entityの位置を設定
   */
  setPosition(id: EntityId, pos: Vec3): void {
    this.getEngine().set_position(id, pos.x, pos.y, pos.z);
  }

  /**
   * Entityの回転を設定（クォータニオン）
   */
  setRotation(id: EntityId, rot: Quat): void {
    this.getEngine().set_rotation(id, rot.x, rot.y, rot.z, rot.w);
  }

  /**
   * Entityのスケールを設定
   */
  setScale(id: EntityId, scale: Vec3): void {
    this.getEngine().set_scale(id, scale.x, scale.y, scale.z);
  }

  /**
   * Entityの位置を取得
   */
  getPosition(id: EntityId): Vec3 | null {
    const arr = this.getEngine().get_position(id);
    return arr ? Vec3Helper.fromArray(arr) : null;
  }

  /**
   * Entityの回転を取得
   */
  getRotation(id: EntityId): Quat | null {
    const arr = this.getEngine().get_rotation(id);
    return arr ? QuatHelper.fromArray(arr) : null;
  }

  /**
   * Entityのスケールを取得
   */
  getScale(id: EntityId): Vec3 | null {
    const arr = this.getEngine().get_scale(id);
    return arr ? Vec3Helper.fromArray(arr) : null;
  }

  /**
   * Entity名を取得
   */
  getName(id: EntityId): string | null {
    return this.getEngine().get_name(id) ?? null;
  }

  /**
   * Entity名を設定
   */
  setName(id: EntityId, name: string): void {
    this.getEngine().set_name(id, name);
    this.entities.set(id, name);
  }

  /**
   * Entityが生存しているか確認
   */
  isAlive(id: EntityId): boolean {
    return this.getEngine().is_alive(id);
  }

  /**
   * EntityのTransformを取得
   */
  getTransform(id: EntityId): Transform | null {
    const position = this.getPosition(id);
    const rotation = this.getRotation(id);
    const scale = this.getScale(id);

    if (!position || !rotation || !scale) {
      return null;
    }

    return { position, rotation, scale };
  }

  /**
   * Entityデータを取得（ID、名前、Transform）
   */
  getEntityData(id: EntityId): EntityData | null {
    const name = this.getName(id);
    const transform = this.getTransform(id);

    if (!name || !transform) {
      return null;
    }

    return { id, name, transform };
  }

  /**
   * 全EntityIDを取得（ローカルキャッシュから）
   */
  getAllEntityIds(): EntityId[] {
    return Array.from(this.entities.keys());
  }

  /**
   * Entity数を取得
   */
  getEntityCount(): number {
    return this.getEngine().entity_count();
  }

  /**
   * フレーム更新（レンダリング）
   */
  tick(deltaTime: number): void {
    this.getEngine().tick(deltaTime);
  }

  /**
   * Canvasをリサイズ
   */
  resize(width: number, height: number): void {
    this.getEngine().resize(width, height);
  }

  /**
   * Canvas幅を取得
   */
  getWidth(): number {
    return this.getEngine().width();
  }

  /**
   * Canvas高さを取得
   */
  getHeight(): number {
    return this.getEngine().height();
  }

  /**
   * 新しいクエリビルダーを作成
   */
  query(): EntityQueryBuilder {
    return new EntityQueryBuilder();
  }

  /**
   * クエリを実行
   */
  executeQuery(query: EntityQueryBuilder | QueryDescriptor): QueryResult {
    const json = query instanceof EntityQueryBuilder
      ? query.toJSON()
      : JSON.stringify(query);

    return this.getEngine().execute_query(json);
  }

  /**
   * クエリを購読
   */
  subscribeQuery(
    query: EntityQueryBuilder | QueryDescriptor,
    callback: (result: QueryResult) => void
  ): number {
    const json = query instanceof EntityQueryBuilder
      ? query.toJSON()
      : JSON.stringify(query);

    return this.getEngine().subscribe_query(json, callback);
  }

  /**
   * 購読解除
   */
  unsubscribeQuery(subscriptionId: number): boolean {
    return this.getEngine().unsubscribe_query(subscriptionId);
  }

  // ========================================================================
  // カメラ操作 API
  // ========================================================================

  /**
   * カメラをターゲット周りで回転（Orbit）
   * @param deltaX 水平方向の回転量（ラジアン）
   * @param deltaY 垂直方向の回転量（ラジアン）
   */
  orbitCamera(deltaX: number, deltaY: number): void {
    this.getEngine().orbit_camera(deltaX, deltaY);
  }

  /**
   * カメラを平行移動（Pan）
   * @param deltaX 右方向への移動量
   * @param deltaY 上方向への移動量
   */
  panCamera(deltaX: number, deltaY: number): void {
    this.getEngine().pan_camera(deltaX, deltaY);
  }

  /**
   * カメラをズーム
   * @param delta 正で近づく、負で遠ざかる
   */
  zoomCamera(delta: number): void {
    this.getEngine().zoom_camera(delta);
  }

  /**
   * カメラターゲットを設定
   */
  setCameraTarget(target: Vec3): void {
    this.getEngine().set_camera_target(target.x, target.y, target.z);
  }

  /**
   * カメラ位置を取得
   */
  getCameraPosition(): Vec3 {
    const arr = this.getEngine().get_camera_position();
    return Vec3Helper.fromArray(arr);
  }

  /**
   * カメラターゲットを取得
   */
  getCameraTarget(): Vec3 {
    const arr = this.getEngine().get_camera_target();
    return Vec3Helper.fromArray(arr);
  }

  /**
   * 特定のEntityにカメラをフォーカス
   */
  focusOnEntity(id: EntityId): void {
    const pos = this.getPosition(id);
    if (pos) {
      this.setCameraTarget(pos);
    }
  }

  // ========================================================================
  // Entity Picking API
  // ========================================================================

  /**
   * スクリーン座標からEntityを取得（レイキャスト）
   * @param screenX スクリーンX座標 (0.0 = 左端, 1.0 = 右端)
   * @param screenY スクリーンY座標 (0.0 = 上端, 1.0 = 下端)
   * @returns 選択されたEntityのID、なければnull
   */
  pickEntity(screenX: number, screenY: number): EntityId | null {
    const result = this.getEngine().pick_entity(screenX, screenY);
    return result >= 0 ? result : null;
  }

  /**
   * スクリーン座標をワールド座標に変換
   * @param screenX スクリーンX座標 (0.0 〜 1.0)
   * @param screenY スクリーンY座標 (0.0 〜 1.0)
   * @param depth 深度値 (0 〜 1, 0が近、1が遠)
   * @returns ワールド座標
   */
  screenToWorld(screenX: number, screenY: number, depth: number): Vec3 {
    const arr = this.getEngine().screen_to_world(screenX, screenY, depth);
    return Vec3Helper.fromArray(arr);
  }

  /**
   * キャンバス座標からEntityをピックする便利メソッド
   */
  pickEntityAtCanvas(canvasX: number, canvasY: number): EntityId | null {
    const width = this.getWidth();
    const height = this.getHeight();
    // Rust側は 0〜1 のスクリーン座標を期待
    const screenX = canvasX / width;
    const screenY = canvasY / height;
    return this.pickEntity(screenX, screenY);
  }

  // ========================================================================
  // Gizmo API
  // ========================================================================

  /**
   * Gizmoモードを設定
   */
  setGizmoMode(mode: GizmoMode): void {
    this.getEngine().set_gizmo_mode(mode);
  }

  /**
   * Gizmoの表示/非表示を設定
   */
  setGizmoVisible(visible: boolean): void {
    this.getEngine().set_gizmo_visible(visible);
  }

  /**
   * Gizmoの位置を設定
   */
  setGizmoPosition(pos: Vec3): void {
    this.getEngine().set_gizmo_position(pos.x, pos.y, pos.z);
  }

  /**
   * Gizmoの回転を設定
   */
  setGizmoRotation(rot: Quat): void {
    this.getEngine().set_gizmo_rotation(rot.x, rot.y, rot.z, rot.w);
  }

  /**
   * ホバー中の軸を設定
   */
  setGizmoHoveredAxis(axis: GizmoAxis): void {
    this.getEngine().set_gizmo_hovered_axis(axis);
  }

  /**
   * アクティブな軸を設定（ドラッグ中）
   */
  setGizmoActiveAxis(axis: GizmoAxis): void {
    this.getEngine().set_gizmo_active_axis(axis);
  }

  /**
   * Gizmoが表示中か確認
   */
  isGizmoVisible(): boolean {
    return this.getEngine().is_gizmo_visible();
  }

  /**
   * 選択中EntityにGizmoを同期
   * @param id EntityId
   * @param space Gizmo座標系 ('world' | 'local')
   */
  syncGizmoToEntity(id: EntityId, space: 'world' | 'local' = 'world'): void {
    const transform = this.getTransform(id);
    if (transform) {
      this.setGizmoPosition(transform.position);
      // World空間モードでは回転を適用しない（常にワールド軸）
      // Local空間モードではEntityの回転に追従
      if (space === 'local') {
        this.setGizmoRotation(transform.rotation);
      } else {
        this.setGizmoRotation({ x: 0, y: 0, z: 0, w: 1 }); // 単位クォータニオン
      }
      this.setGizmoVisible(true);
    }
  }

  /**
   * Gizmoを非表示にする
   */
  hideGizmo(): void {
    this.setGizmoVisible(false);
  }

  // ========================================================================
  // Gizmo Interaction API
  // ========================================================================

  /**
   * Gizmoヒットテスト
   * @param screenX スクリーンX座標 (0.0〜1.0)
   * @param screenY スクリーンY座標 (0.0〜1.0)
   * @returns ヒットした軸 ("x", "y", "z", "xy", "yz", "xz", "all", "")
   */
  gizmoHitTest(screenX: number, screenY: number): GizmoAxis | '' {
    const result = this.getEngine().gizmo_hit_test(screenX, screenY);
    return result as GizmoAxis | '';
  }

  /**
   * Gizmoドラッグ開始
   * @param screenX スクリーンX座標 (0.0〜1.0)
   * @param screenY スクリーンY座標 (0.0〜1.0)
   * @returns ドラッグ開始した軸（""の場合はヒットなし）
   */
  startGizmoDrag(screenX: number, screenY: number): GizmoAxis | '' {
    const result = this.getEngine().start_gizmo_drag(screenX, screenY);
    return result as GizmoAxis | '';
  }

  /**
   * Gizmoドラッグ更新（Translate/Scaleモード）
   * @param screenX スクリーンX座標 (0.0〜1.0)
   * @param screenY スクリーンY座標 (0.0〜1.0)
   * @returns 移動/スケール変化量
   */
  updateGizmoDrag(screenX: number, screenY: number): Vec3 {
    const arr = this.getEngine().update_gizmo_drag(screenX, screenY);
    return Vec3Helper.fromArray(arr);
  }

  /**
   * Gizmoドラッグ更新（Rotateモード）
   * @param screenX スクリーンX座標 (0.0〜1.0)
   * @param screenY スクリーンY座標 (0.0〜1.0)
   * @returns 回転差分（Quaternion）
   */
  updateGizmoDragRotate(screenX: number, screenY: number): Quat {
    const arr = this.getEngine().update_gizmo_drag_rotate(screenX, screenY);
    return QuatHelper.fromArray(arr);
  }

  /**
   * Gizmoドラッグ終了
   */
  endGizmoDrag(): void {
    this.getEngine().end_gizmo_drag();
  }

  /**
   * キャンバス座標からGizmoヒットテスト（便利メソッド）
   */
  gizmoHitTestAtCanvas(canvasX: number, canvasY: number): GizmoAxis | '' {
    const width = this.getWidth();
    const height = this.getHeight();
    const screenX = canvasX / width;
    const screenY = canvasY / height;
    return this.gizmoHitTest(screenX, screenY);
  }

  /**
   * キャンバス座標からGizmoドラッグ開始（便利メソッド）
   */
  startGizmoDragAtCanvas(canvasX: number, canvasY: number): GizmoAxis | '' {
    const width = this.getWidth();
    const height = this.getHeight();
    const screenX = canvasX / width;
    const screenY = canvasY / height;
    return this.startGizmoDrag(screenX, screenY);
  }

  /**
   * キャンバス座標でGizmoドラッグ更新（便利メソッド）
   */
  updateGizmoDragAtCanvas(canvasX: number, canvasY: number): Vec3 {
    const width = this.getWidth();
    const height = this.getHeight();
    const screenX = canvasX / width;
    const screenY = canvasY / height;
    return this.updateGizmoDrag(screenX, screenY);
  }

  /**
   * キャンバス座標でGizmoドラッグ更新（Rotateモード、便利メソッド）
   */
  updateGizmoDragRotateAtCanvas(canvasX: number, canvasY: number): Quat {
    const width = this.getWidth();
    const height = this.getHeight();
    const screenX = canvasX / width;
    const screenY = canvasY / height;
    return this.updateGizmoDragRotate(screenX, screenY);
  }

  /**
   * エンジンを破棄
   */
  dispose(): void {
    if (this.engine) {
      this.engine.free();
      this.engine = null;
      this.initialized = false;
      this.entities.clear();
      console.log('EngineAPI disposed');
    }
  }

  /**
   * エンジンが初期化済みか確認（未初期化時は例外）
   */
  private assertInitialized(): void {
    if (!this.initialized || !this.engine) {
      throw new Error('Engine not initialized. Call initialize() first.');
    }
  }

  /**
   * エンジンインスタンスを取得（未初期化時は例外）
   */
  private getEngine(): WasmEngine {
    this.assertInitialized();
    return this.engine!;
  }
}

// シングルトンインスタンス
let engineInstance: EngineAPI | null = null;

/**
 * シングルトンEngineAPIインスタンスを取得
 */
export function getEngine(): EngineAPI {
  if (!engineInstance) {
    engineInstance = new EngineAPI();
  }
  return engineInstance;
}
