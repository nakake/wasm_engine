import type { ReactNode } from 'react';
import type { Vec3 } from '../../engine/types';

interface PropertyFieldProps {
  label: string;
  children: ReactNode;
}

export function PropertyField({ label, children }: PropertyFieldProps) {
  return (
    <div className="property-field">
      <label className="property-label">{label}</label>
      <div className="property-value">{children}</div>
    </div>
  );
}

// === 数値入力 ===

interface NumberInputProps {
  value: number;
  onChange: (value: number) => void;
  step?: number;
  min?: number;
  max?: number;
}

export function NumberInput({
  value,
  onChange,
  step = 0.1,
  min,
  max,
}: NumberInputProps) {
  return (
    <input
      type="number"
      className="number-input"
      value={value}
      step={step}
      min={min}
      max={max}
      onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
    />
  );
}

// === Vec3入力 ===

interface Vec3InputProps {
  value: Vec3;
  onChange: (value: Vec3) => void;
  step?: number;
}

export function Vec3Input({ value, onChange, step = 0.1 }: Vec3InputProps) {
  return (
    <div className="vec3-input">
      <div className="vec3-field">
        <span className="vec3-label x">X</span>
        <input
          type="number"
          className="number-input"
          value={value.x}
          step={step}
          onChange={(e) =>
            onChange({ ...value, x: parseFloat(e.target.value) || 0 })
          }
        />
      </div>
      <div className="vec3-field">
        <span className="vec3-label y">Y</span>
        <input
          type="number"
          className="number-input"
          value={value.y}
          step={step}
          onChange={(e) =>
            onChange({ ...value, y: parseFloat(e.target.value) || 0 })
          }
        />
      </div>
      <div className="vec3-field">
        <span className="vec3-label z">Z</span>
        <input
          type="number"
          className="number-input"
          value={value.z}
          step={step}
          onChange={(e) =>
            onChange({ ...value, z: parseFloat(e.target.value) || 0 })
          }
        />
      </div>
    </div>
  );
}
