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
 */
export type EntityId = number;

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
