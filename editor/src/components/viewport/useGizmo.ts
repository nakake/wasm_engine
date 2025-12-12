import { useEffect, useCallback } from 'react';
import { useEngineOrNull } from '../../engine/context';
import { useEditorStore, type EditorTool } from '../../stores/editorStore';
import type { GizmoMode } from '../../engine/types';

/**
 * EditorToolをGizmoModeに変換
 */
function toolToGizmoMode(tool: EditorTool): GizmoMode | null {
  switch (tool) {
    case 'move':
      return 'translate';
    case 'rotate':
      return 'rotate';
    case 'scale':
      return 'scale';
    default:
      return null;
  }
}

/**
 * Gizmo表示と選択Entityの同期を管理するフック
 */
export function useGizmo() {
  const engine = useEngineOrNull();
  const selectedEntityIds = useEditorStore((state) => state.selectedEntityIds);
  const activeTool = useEditorStore((state) => state.activeTool);
  const gizmoSpace = useEditorStore((state) => state.gizmoSpace);

  // 選択Entityが変更されたらGizmo位置を更新
  useEffect(() => {
    if (!engine) return;

    // 選択なし → Gizmo非表示
    if (selectedEntityIds.length === 0) {
      engine.hideGizmo();
      return;
    }

    // 単一選択 → その Entity の位置に Gizmo を表示
    if (selectedEntityIds.length === 1) {
      const id = selectedEntityIds[0];
      if (engine.isAlive(id)) {
        engine.syncGizmoToEntity(id, gizmoSpace);
      } else {
        engine.hideGizmo();
      }
      return;
    }

    // 複数選択 → 選択中心にGizmoを表示（将来拡張）
    // 現在は最初のEntityの位置を使用
    const firstId = selectedEntityIds[0];
    if (engine.isAlive(firstId)) {
      engine.syncGizmoToEntity(firstId, gizmoSpace);
    }
  }, [engine, selectedEntityIds, gizmoSpace]);

  // アクティブツールが変更されたらGizmoモードを更新
  useEffect(() => {
    if (!engine) return;

    const gizmoMode = toolToGizmoMode(activeTool);

    if (gizmoMode) {
      engine.setGizmoMode(gizmoMode);
      // Entityが選択されていればGizmoを表示
      if (selectedEntityIds.length > 0) {
        engine.setGizmoVisible(true);
      }
    } else {
      // select ツールの場合はGizmoを非表示
      engine.hideGizmo();
    }
  }, [engine, activeTool, selectedEntityIds.length]);

  // Gizmo軸のホバー状態を設定
  const setHoveredAxis = useCallback(
    (axis: 'none' | 'x' | 'y' | 'z' | 'xy' | 'yz' | 'xz' | 'all') => {
      if (engine) {
        engine.setGizmoHoveredAxis(axis);
      }
    },
    [engine]
  );

  // Gizmoドラッグ開始時のアクティブ軸設定
  const setActiveAxis = useCallback(
    (axis: 'none' | 'x' | 'y' | 'z' | 'xy' | 'yz' | 'xz' | 'all') => {
      if (engine) {
        engine.setGizmoActiveAxis(axis);
      }
    },
    [engine]
  );

  // Gizmo位置を更新（ドラッグ中のEntity移動後など）
  const refreshGizmoPosition = useCallback(() => {
    if (!engine || selectedEntityIds.length === 0) return;

    const id = selectedEntityIds[0];
    if (engine.isAlive(id)) {
      engine.syncGizmoToEntity(id, gizmoSpace);
    }
  }, [engine, selectedEntityIds, gizmoSpace]);

  return {
    setHoveredAxis,
    setActiveAxis,
    refreshGizmoPosition,
  };
}
