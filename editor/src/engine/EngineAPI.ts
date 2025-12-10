import type { EntityId, Vec3, Quat, EntityData, Transform, QueryDescriptor, QueryResult } from './types';
import { Vec3 as Vec3Helper, Quat as QuatHelper } from './types';
import { EntityQueryBuilder } from './query';

// WASMエンジン型（wasm-packで生成される）
interface WasmEngine {
  create_entity(name: string): number;
  delete_entity(id: number): boolean;
  set_position(id: number, x: number, y: number, z: number): void;
  set_rotation(id: number, x: number, y: number, z: number, w: number): void;
  set_scale(id: number, x: number, y: number, z: number): void;
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
