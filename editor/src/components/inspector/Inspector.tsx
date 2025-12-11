import { useState, useEffect } from 'react';
import { useEditorStore, useSelectedEntity } from '../../stores/editorStore';
import { useEngine } from '../../engine/context';
import { Panel } from '../layout/Panel';
import { TransformEditor } from './TransformEditor';

export function Inspector() {
  const togglePanel = useEditorStore((s) => s.togglePanel);
  const selectedId = useSelectedEntity();
  const selectedIds = useEditorStore((s) => s.selectedEntityIds);
  const engine = useEngine();

  // Entity名
  const [name, setName] = useState('');

  // 選択変更時に名前を同期
  useEffect(() => {
    if (selectedId !== null) {
      const entityName = engine.getName(selectedId);
      setName(entityName ?? '');
    }
  }, [selectedId, engine]);

  // 名前変更ハンドラ
  const handleNameChange = (newName: string) => {
    setName(newName);
    if (selectedId !== null) {
      engine.setName(selectedId, newName);
    }
  };

  // 複数選択時
  if (selectedIds.length > 1) {
    return (
      <Panel title="Inspector" onClose={() => togglePanel('inspector')}>
        <div className="inspector-multi">{selectedIds.length} entities selected</div>
      </Panel>
    );
  }

  // 未選択時
  if (selectedId === null) {
    return (
      <Panel title="Inspector" onClose={() => togglePanel('inspector')}>
        <div className="inspector-empty">No entity selected</div>
      </Panel>
    );
  }

  return (
    <Panel title="Inspector" onClose={() => togglePanel('inspector')}>
      <div className="inspector">
        {/* Entity名 */}
        <div className="inspector-header">
          <input
            className="inspector-name"
            value={name}
            onChange={(e) => handleNameChange(e.target.value)}
            placeholder="Entity Name"
          />
          <span className="inspector-id">ID: {selectedId}</span>
        </div>

        {/* Transform */}
        <TransformEditor entityId={selectedId} />

        {/* コンポーネントリスト（将来拡張） */}
        <div className="inspector-section">
          <div className="inspector-section-header">Components</div>
          <div className="inspector-components">
            <span className="inspector-component-tag">Transform</span>
            <span className="inspector-component-tag">Name</span>
          </div>
        </div>
      </div>
    </Panel>
  );
}
