import { useEditorStore } from '../../stores/editorStore';
import { EntityId } from '../../engine/types';

interface HierarchyItemProps {
  id: number;
  name: string;
  depth?: number;
}

export function HierarchyItem({ id, name, depth = 0 }: HierarchyItemProps) {
  const selectedIds = useEditorStore((s) => s.selectedEntityIds);
  const selectEntity = useEditorStore((s) => s.selectEntity);
  const toggleEntitySelection = useEditorStore((s) => s.toggleEntitySelection);

  const isSelected = selectedIds.includes(id);

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (e.ctrlKey || e.metaKey) {
      // Ctrl/Cmd + クリックで選択をトグル
      toggleEntitySelection(id);
    } else {
      // 通常クリックで単一選択
      selectEntity(id);
    }
  };

  const handleDoubleClick = () => {
    // 将来: カメラをEntityにフォーカス
    console.log('Focus on entity:', id);
  };

  return (
    <div
      className={`hierarchy-item ${isSelected ? 'selected' : ''}`}
      style={{ paddingLeft: `${12 + depth * 16}px` }}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
    >
      {/* Entity名 */}
      <span className="hierarchy-name">{name}</span>

      {/* Entity ID (index形式で表示) */}
      <span className="hierarchy-id">#{EntityId.index(id)}</span>
    </div>
  );
}
