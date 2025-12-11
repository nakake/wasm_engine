import { useEditorStore, type EditorTool } from '../../stores/editorStore';

const TOOLS: { id: EditorTool; label: string; shortcut: string }[] = [
  { id: 'select', label: 'Select', shortcut: 'Q' },
  { id: 'move', label: 'Move', shortcut: 'W' },
  { id: 'rotate', label: 'Rotate', shortcut: 'E' },
  { id: 'scale', label: 'Scale', shortcut: 'R' },
];

export function Toolbar() {
  const activeTool = useEditorStore((s) => s.activeTool);
  const gizmoSpace = useEditorStore((s) => s.gizmoSpace);
  const setActiveTool = useEditorStore((s) => s.setActiveTool);
  const setGizmoSpace = useEditorStore((s) => s.setGizmoSpace);
  const togglePanel = useEditorStore((s) => s.togglePanel);

  return (
    <div className="toolbar">
      {/* ツール選択 */}
      <div className="toolbar-group">
        {TOOLS.map((tool) => (
          <button
            key={tool.id}
            className={`toolbar-btn ${activeTool === tool.id ? 'active' : ''}`}
            onClick={() => setActiveTool(tool.id)}
            title={`${tool.label} (${tool.shortcut})`}
          >
            {tool.label}
          </button>
        ))}
      </div>

      <div className="toolbar-separator" />

      {/* Gizmo空間 */}
      <div className="toolbar-group">
        <button
          className={`toolbar-btn ${gizmoSpace === 'local' ? 'active' : ''}`}
          onClick={() => setGizmoSpace('local')}
          title="Local Space"
        >
          Local
        </button>
        <button
          className={`toolbar-btn ${gizmoSpace === 'world' ? 'active' : ''}`}
          onClick={() => setGizmoSpace('world')}
          title="World Space"
        >
          World
        </button>
      </div>

      {/* スペーサー */}
      <div className="toolbar-spacer" />

      {/* パネルトグル */}
      <div className="toolbar-group">
        <button
          className="toolbar-btn"
          onClick={() => togglePanel('hierarchy')}
          title="Toggle Hierarchy Panel"
        >
          Hierarchy
        </button>
        <button
          className="toolbar-btn"
          onClick={() => togglePanel('inspector')}
          title="Toggle Inspector Panel"
        >
          Inspector
        </button>
      </div>
    </div>
  );
}
