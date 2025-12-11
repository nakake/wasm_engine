import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';

// === 型定義 ===

export type EditorTool = 'select' | 'move' | 'rotate' | 'scale';
export type GizmoSpace = 'local' | 'world';

interface EditorState {
  // 選択状態
  selectedEntityIds: number[];

  // ツール状態
  activeTool: EditorTool;
  gizmoSpace: GizmoSpace;

  // UI状態
  isHierarchyVisible: boolean;
  isInspectorVisible: boolean;

  // アクション
  selectEntity: (id: number, addToSelection?: boolean) => void;
  selectEntities: (ids: number[]) => void;
  deselectAll: () => void;
  toggleEntitySelection: (id: number) => void;
  setActiveTool: (tool: EditorTool) => void;
  setGizmoSpace: (space: GizmoSpace) => void;
  togglePanel: (panel: 'hierarchy' | 'inspector') => void;
}

// === Store ===

export const useEditorStore = create<EditorState>()(
  subscribeWithSelector((set) => ({
    // 初期状態
    selectedEntityIds: [],
    activeTool: 'select',
    gizmoSpace: 'world',
    isHierarchyVisible: true,
    isInspectorVisible: true,

    // === 選択アクション ===

    selectEntity: (id, addToSelection = false) => {
      set((state) => ({
        selectedEntityIds: addToSelection
          ? state.selectedEntityIds.includes(id)
            ? state.selectedEntityIds // 既に選択済み
            : [...state.selectedEntityIds, id]
          : [id],
      }));
    },

    selectEntities: (ids) => {
      set({ selectedEntityIds: ids });
    },

    deselectAll: () => {
      set({ selectedEntityIds: [] });
    },

    toggleEntitySelection: (id) => {
      set((state) => ({
        selectedEntityIds: state.selectedEntityIds.includes(id)
          ? state.selectedEntityIds.filter((eid) => eid !== id)
          : [...state.selectedEntityIds, id],
      }));
    },

    // === ツールアクション ===

    setActiveTool: (tool) => {
      set({ activeTool: tool });
    },

    setGizmoSpace: (space) => {
      set({ gizmoSpace: space });
    },

    // === UIアクション ===

    togglePanel: (panel) => {
      set((state) => ({
        isHierarchyVisible:
          panel === 'hierarchy'
            ? !state.isHierarchyVisible
            : state.isHierarchyVisible,
        isInspectorVisible:
          panel === 'inspector'
            ? !state.isInspectorVisible
            : state.isInspectorVisible,
      }));
    },
  }))
);

// === セレクター（派生状態） ===

/** 単一選択時のEntityId取得（複数選択時はnull） */
export const useSelectedEntity = () => {
  const ids = useEditorStore((state) => state.selectedEntityIds);
  return ids.length === 1 ? ids[0] : null;
};

/** 複数選択判定 */
export const useHasMultipleSelection = () => {
  return useEditorStore((state) => state.selectedEntityIds.length > 1);
};

/** 選択中か判定 */
export const useIsEntitySelected = (id: number) => {
  return useEditorStore((state) => state.selectedEntityIds.includes(id));
};

/** 選択数 */
export const useSelectionCount = () => {
  return useEditorStore((state) => state.selectedEntityIds.length);
};

// デバッグ用にグローバル公開
if (typeof window !== 'undefined') {
  (window as unknown as { editorStore: typeof useEditorStore }).editorStore = useEditorStore;
}
