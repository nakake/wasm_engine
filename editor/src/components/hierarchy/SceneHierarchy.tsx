import { useEngine } from '../../engine/context';
import { useEntityList } from '../../engine/hooks';
import { EntityId } from '../../engine/types';
import { useEditorStore } from '../../stores/editorStore';
import { Panel } from '../layout/Panel';
import { HierarchyItem } from './HierarchyItem';

export function SceneHierarchy() {
  const engine = useEngine();
  const togglePanel = useEditorStore((s) => s.togglePanel);
  const deselectAll = useEditorStore((s) => s.deselectAll);

  // 全Entityを取得
  const { data, loading, error } = useEntityList();

  const handleCreateEntity = () => {
    const name = `Entity_${Date.now() % 10000}`;
    engine.createEntity(name);
  };

  const handleBackgroundClick = () => {
    deselectAll();
  };

  // エンティティリストを整形
  const entities = data?.rows ?? [];

  return (
    <Panel title="Hierarchy" onClose={() => togglePanel('hierarchy')}>
      <div className="hierarchy" onClick={handleBackgroundClick}>
        {/* 新規Entity作成ボタン */}
        <div className="hierarchy-actions" onClick={(e) => e.stopPropagation()}>
          <button className="hierarchy-add-btn" onClick={handleCreateEntity}>
            + Create Entity
          </button>
        </div>

        {/* Entityリスト */}
        <div className="hierarchy-list">
          {loading && <div className="hierarchy-empty">Loading...</div>}

          {error && (
            <div className="hierarchy-empty">Error: {error.message}</div>
          )}

          {!loading && !error && entities.length === 0 && (
            <div className="hierarchy-empty">No entities in scene</div>
          )}

          {!loading &&
            !error &&
            entities.map((entity) => (
              <HierarchyItem
                key={entity.id}
                id={entity.id}
                name={(entity.fields.name as string) ?? `Entity ${EntityId.index(entity.id)}`}
              />
            ))}
        </div>
      </div>
    </Panel>
  );
}
