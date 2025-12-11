/**
 * EngineContext - ReactコンポーネントからEngineAPIにアクセスするためのContext
 */

import { createContext, useContext, createElement } from 'react';
import type { ReactNode } from 'react';
import type { EngineAPI } from './EngineAPI';

const EngineContext = createContext<EngineAPI | null>(null);

export interface EngineProviderProps {
  engine: EngineAPI | null;
  children: ReactNode;
}

/**
 * EngineAPIをReactツリーに提供するProvider
 */
export function EngineProvider({ engine, children }: EngineProviderProps) {
  return createElement(EngineContext.Provider, { value: engine }, children);
}

/**
 * EngineAPIを取得するフック
 * @throws EngineProvider外で使用した場合
 */
export function useEngine(): EngineAPI {
  const engine = useContext(EngineContext);
  if (!engine) {
    throw new Error('useEngine must be used within EngineProvider');
  }
  return engine;
}

/**
 * EngineAPIを取得するフック（null許容版）
 * engineが未初期化の場合はnullを返す
 */
export function useEngineOrNull(): EngineAPI | null {
  return useContext(EngineContext);
}
