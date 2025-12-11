/**
 * 3Dベクトル型
 */
export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

/**
 * 回転用クォータニオン型
 */
export interface Quat {
  x: number;
  y: number;
  z: number;
  w: number;
}

/**
 * Entity識別子（Rustのu32）
 * packed format: (generation << 20) | index
 */
export type EntityId = number;

/**
 * EntityIdヘルパー関数
 */
export const EntityId = {
  /** Packed IDからindexを取得 */
  index: (id: EntityId): number => id & 0xFFFFF,
  /** Packed IDからgenerationを取得 */
  generation: (id: EntityId): number => id >> 20,
  /** index と generation から packed ID を作成 */
  pack: (index: number, generation: number): EntityId => (generation << 20) | index,
};

/**
 * Transformコンポーネントデータ
 */
export interface Transform {
  position: Vec3;
  rotation: Quat;
  scale: Vec3;
}

/**
 * Entityデータ（階層表示用）
 */
export interface EntityData {
  id: EntityId;
  name: string;
  transform: Transform;
}

/**
 * Vec3ヘルパー関数
 */
export const Vec3 = {
  zero: (): Vec3 => ({ x: 0, y: 0, z: 0 }),
  one: (): Vec3 => ({ x: 1, y: 1, z: 1 }),
  create: (x: number, y: number, z: number): Vec3 => ({ x, y, z }),
  fromArray: (arr: number[]): Vec3 => ({ x: arr[0], y: arr[1], z: arr[2] }),
};

/**
 * Quatヘルパー関数
 */
export const Quat = {
  identity: (): Quat => ({ x: 0, y: 0, z: 0, w: 1 }),
  create: (x: number, y: number, z: number, w: number): Quat => ({ x, y, z, w }),
  fromArray: (arr: number[]): Quat => ({ x: arr[0], y: arr[1], z: arr[2], w: arr[3] }),
};

// ========== Query Types ==========

/**
 * 比較演算子
 */
export type CompareOp = '==' | '!=' | '<' | '<=' | '>' | '>=';

/**
 * ソート方向
 */
export type SortDirection = 'asc' | 'desc';

/**
 * フィルター条件
 */
export interface FilterExpr {
  field: string;
  op: CompareOp;
  value: number | string | boolean | null;
}

/**
 * ソート条件
 */
export interface OrderBy {
  field: string;
  direction: SortDirection;
}

/**
 * クエリ記述子
 */
export interface QueryDescriptor {
  select: string[];
  with_components: string[];
  without_components: string[];
  filters: FilterExpr[];
  order_by: OrderBy | null;
  limit: number | null;
  offset?: number | null;
}

/**
 * クエリ結果の1行
 */
export interface QueryResultRow {
  id: number;
  fields: Record<string, unknown>;
}

/**
 * クエリ結果
 */
export interface QueryResult {
  rows: QueryResultRow[];
  total_count: number;
}

// ========== Gizmo Types ==========

/**
 * Gizmoモード
 */
export type GizmoMode = 'translate' | 'rotate' | 'scale';

/**
 * Gizmo軸
 */
export type GizmoAxis = 'none' | 'x' | 'y' | 'z' | 'xy' | 'yz' | 'xz' | 'all';
