import { useState, useEffect, useCallback } from 'react';
import { useEngine } from '../../engine/context';
import { Vec3Input } from './PropertyField';
import type { Vec3, Quat } from '../../engine/types';

interface TransformEditorProps {
  entityId: number;
}

// Quaternion → オイラー角（度）変換
function quatToEuler(q: Quat): Vec3 {
  const { x, y, z, w } = q;

  // Roll (X)
  const sinr_cosp = 2 * (w * x + y * z);
  const cosr_cosp = 1 - 2 * (x * x + y * y);
  const roll = Math.atan2(sinr_cosp, cosr_cosp);

  // Pitch (Y)
  const sinp = 2 * (w * y - z * x);
  let pitch: number;
  if (Math.abs(sinp) >= 1) {
    pitch = (Math.PI / 2) * Math.sign(sinp);
  } else {
    pitch = Math.asin(sinp);
  }

  // Yaw (Z)
  const siny_cosp = 2 * (w * z + x * y);
  const cosy_cosp = 1 - 2 * (y * y + z * z);
  const yaw = Math.atan2(siny_cosp, cosy_cosp);

  // ラジアン → 度
  const toDeg = 180 / Math.PI;
  return {
    x: roll * toDeg,
    y: pitch * toDeg,
    z: yaw * toDeg,
  };
}

// オイラー角（度）→ Quaternion変換
function eulerToQuat(euler: Vec3): Quat {
  const toRad = Math.PI / 180;
  const cx = Math.cos((euler.x * toRad) / 2);
  const sx = Math.sin((euler.x * toRad) / 2);
  const cy = Math.cos((euler.y * toRad) / 2);
  const sy = Math.sin((euler.y * toRad) / 2);
  const cz = Math.cos((euler.z * toRad) / 2);
  const sz = Math.sin((euler.z * toRad) / 2);

  return {
    x: sx * cy * cz - cx * sy * sz,
    y: cx * sy * cz + sx * cy * sz,
    z: cx * cy * sz - sx * sy * cz,
    w: cx * cy * cz + sx * sy * sz,
  };
}

export function TransformEditor({ entityId }: TransformEditorProps) {
  const engine = useEngine();

  // ローカル状態
  const [position, setPosition] = useState<Vec3>({ x: 0, y: 0, z: 0 });
  const [rotation, setRotation] = useState<Vec3>({ x: 0, y: 0, z: 0 }); // オイラー角
  const [scale, setScale] = useState<Vec3>({ x: 1, y: 1, z: 1 });

  // Entityからデータを同期
  useEffect(() => {
    const pos = engine.getPosition(entityId);
    const rot = engine.getRotation(entityId);
    const scl = engine.getScale(entityId);

    if (pos) setPosition(pos);
    if (rot) setRotation(quatToEuler(rot));
    if (scl) setScale(scl);
  }, [engine, entityId]);

  // Position変更
  const handlePositionChange = useCallback(
    (value: Vec3) => {
      setPosition(value);
      engine.setPosition(entityId, value);
    },
    [engine, entityId]
  );

  // Rotation変更（オイラー角）
  const handleRotationChange = useCallback(
    (value: Vec3) => {
      setRotation(value);
      const quat = eulerToQuat(value);
      engine.setRotation(entityId, quat);
    },
    [engine, entityId]
  );

  // Scale変更
  const handleScaleChange = useCallback(
    (value: Vec3) => {
      setScale(value);
      engine.setScale(entityId, value);
    },
    [engine, entityId]
  );

  return (
    <div className="transform-editor">
      <div className="inspector-section-header">Transform</div>

      <div className="transform-fields">
        <div className="transform-row">
          <label className="transform-label">Position</label>
          <Vec3Input value={position} onChange={handlePositionChange} />
        </div>

        <div className="transform-row">
          <label className="transform-label">Rotation</label>
          <Vec3Input value={rotation} onChange={handleRotationChange} step={1} />
        </div>

        <div className="transform-row">
          <label className="transform-label">Scale</label>
          <Vec3Input value={scale} onChange={handleScaleChange} />
        </div>
      </div>
    </div>
  );
}
