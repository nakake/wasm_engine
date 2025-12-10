/**
 * Engine関連のReactフック
 */

import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import type { QueryResult, QueryDescriptor, QueryResultRow } from './types';
import { EntityQueryBuilder } from './query';
import { useEngine } from './context';

type QueryFactory = () => EntityQueryBuilder | QueryDescriptor;

export interface UseEntityQueryOptions {
  /** 自動購読を無効化（手動で refetch を呼ぶ） */
  manual?: boolean;
  /** 初期データ */
  initialData?: QueryResult;
  /** エラー時のコールバック */
  onError?: (error: Error) => void;
}

export interface UseEntityQueryResult {
  /** クエリ結果 */
  data: QueryResult | null;
  /** ローディング状態 */
  loading: boolean;
  /** エラー */
  error: Error | null;
  /** 手動再実行 */
  refetch: () => void;
}

/**
 * Entityクエリを実行・購読するフック
 *
 * @param queryFactory - クエリを返す関数
 * @param deps - クエリを再構築する依存配列
 * @param options - オプション
 *
 * @example
 * const { data, loading } = useEntityQuery(
 *   () => engine.query()
 *     .select('name', 'position')
 *     .with('Transform')
 *     .where('position.x', '>', 0),
 *   []
 * );
 */
export function useEntityQuery(
  queryFactory: QueryFactory,
  deps: React.DependencyList = [],
  options: UseEntityQueryOptions = {}
): UseEntityQueryResult {
  const engine = useEngine();
  const [data, setData] = useState<QueryResult | null>(options.initialData ?? null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const subscriptionRef = useRef<number | null>(null);
  const onErrorRef = useRef(options.onError);

  // onErrorを最新に保つ
  useEffect(() => {
    onErrorRef.current = options.onError;
  }, [options.onError]);

  // クエリ実行
  const executeQuery = useCallback(() => {
    try {
      const query = queryFactory();
      const result = engine.executeQuery(query);
      setData(result);
      setError(null);
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      setError(err);
      onErrorRef.current?.(err);
    } finally {
      setLoading(false);
    }
  }, [engine, queryFactory]);

  // 購読セットアップ
  useEffect(() => {
    if (options.manual) {
      // 手動モード: 初回実行のみ
      executeQuery();
      return;
    }

    setLoading(true);

    try {
      const query = queryFactory();

      // 購読開始
      subscriptionRef.current = engine.subscribeQuery(query, (result) => {
        setData(result);
        setLoading(false);
        setError(null);
      });
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      setError(err);
      setLoading(false);
      onErrorRef.current?.(err);
    }

    // クリーンアップ
    return () => {
      if (subscriptionRef.current !== null) {
        engine.unsubscribeQuery(subscriptionRef.current);
        subscriptionRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps); // depsが変わったらクエリ再構築

  return {
    data,
    loading,
    error,
    refetch: executeQuery,
  };
}

export interface UseEntityListOptions {
  withComponents?: string[];
  withoutComponents?: string[];
  limit?: number;
}

/**
 * 全Entityのリストを取得するシンプルなフック
 */
export function useEntityList(options: UseEntityListOptions = {}): UseEntityQueryResult {
  const engine = useEngine();

  // optionsをメモ化してdeps用の文字列を生成
  const depsKey = useMemo(
    () =>
      `${options.withComponents?.join(',') ?? ''}_${options.withoutComponents?.join(',') ?? ''}_${options.limit ?? ''}`,
    [options.withComponents, options.withoutComponents, options.limit]
  );

  return useEntityQuery(
    () => {
      let builder = engine.query().select('id', 'name', 'position', 'rotation', 'scale');

      if (options.withComponents) {
        builder = builder.with(...options.withComponents);
      }
      if (options.withoutComponents) {
        builder = builder.without(...options.withoutComponents);
      }
      if (options.limit) {
        builder = builder.limit(options.limit);
      }

      return builder;
    },
    [depsKey]
  );
}

export interface UseEntityResult {
  entity: QueryResultRow | null;
  loading: boolean;
  error: Error | null;
}

/**
 * 単一のEntityデータを取得するフック
 */
export function useEntity(entityId: number | null): UseEntityResult {
  const engine = useEngine();
  const [entity, setEntity] = useState<QueryResultRow | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (entityId === null) {
      setEntity(null);
      setLoading(false);
      return;
    }

    try {
      const name = engine.getName(entityId);
      const position = engine.getPosition(entityId);
      const rotation = engine.getRotation(entityId);
      const scale = engine.getScale(entityId);

      if (name !== null) {
        setEntity({
          id: entityId,
          fields: {
            name,
            position,
            rotation,
            scale,
          },
        });
      } else {
        setEntity(null);
      }
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setLoading(false);
    }
  }, [entityId, engine]);

  return { entity, loading, error };
}
