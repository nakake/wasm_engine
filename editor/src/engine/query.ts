/**
 * EntityQueryBuilder - SQLライクなクエリを構築するビルダークラス
 */

import type {
  QueryDescriptor,
  CompareOp,
  SortDirection,
} from './types';

export class EntityQueryBuilder {
  private descriptor: QueryDescriptor;

  constructor() {
    this.descriptor = {
      select: [],
      with_components: [],
      without_components: [],
      filters: [],
      order_by: null,
      limit: null,
    };
  }

  /**
   * 取得するフィールドを指定
   * @example query.select('name', 'position', 'health')
   */
  select(...fields: string[]): this {
    this.descriptor.select = fields;
    return this;
  }

  /**
   * 必須コンポーネントを指定
   * @example query.with('Transform', 'Enemy')
   */
  with(...components: string[]): this {
    this.descriptor.with_components.push(...components);
    return this;
  }

  /**
   * 除外コンポーネントを指定
   * @example query.without('Dead', 'Inactive')
   */
  without(...components: string[]): this {
    this.descriptor.without_components.push(...components);
    return this;
  }

  /**
   * フィルター条件を追加
   * @example query.where('health', '<', 50)
   */
  where(field: string, op: CompareOp, value: number | string | boolean | null): this {
    this.descriptor.filters.push({ field, op, value });
    return this;
  }

  /**
   * ソート条件を指定
   * @example query.orderBy('name', 'asc')
   */
  orderBy(field: string, direction: SortDirection = 'asc'): this {
    this.descriptor.order_by = { field, direction };
    return this;
  }

  /**
   * 取得件数の上限を指定
   * @example query.limit(10)
   */
  limit(n: number): this {
    this.descriptor.limit = n;
    return this;
  }

  /**
   * スキップ件数を指定
   * @example query.offset(20)
   */
  offset(n: number): this {
    this.descriptor.offset = n;
    return this;
  }

  /**
   * ページネーション
   * @example query.page(2, 20) // 3ページ目、1ページ20件
   */
  page(pageIndex: number, perPage: number): this {
    this.descriptor.offset = pageIndex * perPage;
    this.descriptor.limit = perPage;
    return this;
  }

  /**
   * QueryDescriptor を構築
   */
  build(): QueryDescriptor {
    return { ...this.descriptor };
  }

  /**
   * JSON文字列を構築（WASM API用）
   */
  toJSON(): string {
    return JSON.stringify(this.descriptor);
  }
}
