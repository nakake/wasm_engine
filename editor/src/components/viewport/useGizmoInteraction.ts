import { useEffect, useRef, useCallback } from 'react';
import { useEngineOrNull } from '../../engine/context';
import { useEditorStore } from '../../stores/editorStore';
import type { GizmoAxis } from '../../engine/types';

/**
 * Gizmo操作（ドラッグ）を管理するフック
 */
export function useGizmoInteraction(
  canvasRef: React.RefObject<HTMLCanvasElement | null>
) {
  const engine = useEngineOrNull();
  const selectedEntityIds = useEditorStore((state) => state.selectedEntityIds);
  const activeTool = useEditorStore((state) => state.activeTool);

  const isDraggingGizmo = useRef(false);
  const dragAxis = useRef<GizmoAxis | ''>('');

  // スクリーン座標を取得
  const getCanvasCoords = useCallback(
    (e: MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return { x: 0, y: 0 };
      const rect = canvas.getBoundingClientRect();
      return {
        x: e.clientX - rect.left,
        y: e.clientY - rect.top,
      };
    },
    [canvasRef]
  );

  // マウス移動
  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!engine || activeTool === 'select') return;

      const { x, y } = getCanvasCoords(e);

      if (isDraggingGizmo.current && selectedEntityIds.length > 0) {
        // ドラッグ中: Transform更新
        const entityId = selectedEntityIds[0];

        if (activeTool === 'move') {
          const delta = engine.updateGizmoDragAtCanvas(x, y);
          if (delta.x !== 0 || delta.y !== 0 || delta.z !== 0) {
            const currentPos = engine.getPosition(entityId);
            if (currentPos) {
              engine.setPosition(entityId, {
                x: currentPos.x + delta.x,
                y: currentPos.y + delta.y,
                z: currentPos.z + delta.z,
              });
              // Gizmo位置も更新
              engine.syncGizmoToEntity(entityId);
            }
          }
        } else if (activeTool === 'scale') {
          const delta = engine.updateGizmoDragAtCanvas(x, y);
          if (delta.x !== 0 || delta.y !== 0 || delta.z !== 0) {
            const currentScale = engine.getScale(entityId);
            if (currentScale) {
              engine.setScale(entityId, {
                x: Math.max(0.01, currentScale.x + delta.x),
                y: Math.max(0.01, currentScale.y + delta.y),
                z: Math.max(0.01, currentScale.z + delta.z),
              });
            }
          }
        } else if (activeTool === 'rotate') {
          const deltaRot = engine.updateGizmoDragRotateAtCanvas(x, y);
          // 回転が有効かチェック
          if (deltaRot.w !== 1 || deltaRot.x !== 0 || deltaRot.y !== 0 || deltaRot.z !== 0) {
            const currentRot = engine.getRotation(entityId);
            if (currentRot) {
              // Quaternion乗算 (deltaRot * currentRot)
              const newRot = multiplyQuaternions(deltaRot, currentRot);
              engine.setRotation(entityId, newRot);
              engine.syncGizmoToEntity(entityId);
            }
          }
        }
      } else {
        // ホバー検出
        const axis = engine.gizmoHitTestAtCanvas(x, y);
        engine.setGizmoHoveredAxis(axis || 'none');

        // カーソル変更
        const canvas = canvasRef.current;
        if (canvas) {
          canvas.style.cursor = axis ? 'grab' : 'default';
        }
      }
    },
    [engine, selectedEntityIds, activeTool, getCanvasCoords, canvasRef]
  );

  // マウスダウン
  const handleMouseDown = useCallback(
    (e: MouseEvent) => {
      if (!engine || e.button !== 0 || activeTool === 'select') return;

      const { x, y } = getCanvasCoords(e);
      const axis = engine.startGizmoDragAtCanvas(x, y);

      if (axis) {
        isDraggingGizmo.current = true;
        dragAxis.current = axis;
        const canvas = canvasRef.current;
        if (canvas) {
          canvas.style.cursor = 'grabbing';
        }
        e.preventDefault();
        e.stopPropagation();
      }
    },
    [engine, activeTool, getCanvasCoords, canvasRef]
  );

  // マウスアップ
  const handleMouseUp = useCallback(() => {
    if (!engine) return;

    if (isDraggingGizmo.current) {
      engine.endGizmoDrag();
      isDraggingGizmo.current = false;
      dragAxis.current = '';
      const canvas = canvasRef.current;
      if (canvas) {
        canvas.style.cursor = 'default';
      }
    }
  }, [engine, canvasRef]);

  // イベントリスナー登録
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('mousedown', handleMouseDown, { capture: true });
    canvas.addEventListener('mouseup', handleMouseUp);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      canvas.removeEventListener('mousemove', handleMouseMove);
      canvas.removeEventListener('mousedown', handleMouseDown, { capture: true });
      canvas.removeEventListener('mouseup', handleMouseUp);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [canvasRef, handleMouseMove, handleMouseDown, handleMouseUp]);

  return {
    isDraggingGizmo: () => isDraggingGizmo.current,
  };
}

/**
 * Quaternion乗算
 */
function multiplyQuaternions(
  a: { x: number; y: number; z: number; w: number },
  b: { x: number; y: number; z: number; w: number }
): { x: number; y: number; z: number; w: number } {
  return {
    x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
    y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
    z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
  };
}
