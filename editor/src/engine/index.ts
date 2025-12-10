// Types
export type { Vec3, Quat, EntityId, Transform, EntityData } from './types';
export { Vec3 as Vec3Helper, Quat as QuatHelper } from './types';

// Query Types
export type {
  CompareOp,
  SortDirection,
  FilterExpr,
  OrderBy,
  QueryDescriptor,
  QueryResultRow,
  QueryResult,
} from './types';

// Query Builder
export { EntityQueryBuilder } from './query';

// Context
export { EngineProvider, useEngine } from './context';
export type { EngineProviderProps } from './context';

// Hooks
export {
  useEntityQuery,
  useEntityList,
  useEntity,
} from './hooks';
export type {
  UseEntityQueryOptions,
  UseEntityQueryResult,
  UseEntityListOptions,
  UseEntityResult,
} from './hooks';

// API
export { EngineAPI, getEngine } from './EngineAPI';
