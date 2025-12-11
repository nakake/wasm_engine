import { useEffect, useRef, useCallback } from 'react';
import { useEngineOrNull } from '../../engine/context';
import { useEditorStore } from '../../stores';

export interface ViewportControlsOptions {
  /** Orbit回転速度（デフォルト: 0.01） */
  orbitSpeed?: number;
  /** Pan移動速度（デフォルト: 0.01） */
  panSpeed?: number;
  /** Zoom速度（デフォルト: 0.1） */
  zoomSpeed?: number;
  /** クリック選択を有効にするか（デフォルト: true） */
  enablePicking?: boolean;
}

/**
 * Viewportカメラ操作フック
 *
 * - 左クリック: Entity選択（Picking）
 * - 右ドラッグ: Orbit（カメラ回転）
 * - 中ドラッグ / Shift+右ドラッグ: Pan（平行移動）
 * - ホイール: Zoom（ズーム）
 * - F キー: 選択Entityにフォーカス
 */
export function useViewportControls(
  canvasRef: React.RefObject<HTMLCanvasElement | null>,
  options: ViewportControlsOptions = {}
) {
  const engine = useEngineOrNull();
  const selectedEntityIds = useEditorStore((state) => state.selectedEntityIds);
  const selectEntity = useEditorStore((state) => state.selectEntity);
  const deselectAll = useEditorStore((state) => state.deselectAll);

  const isDragging = useRef(false);
  const hasDragged = useRef(false); // ドラッグしたかどうか（クリック判定用）
  const lastPos = useRef({ x: 0, y: 0 });
  const mouseDownPos = useRef({ x: 0, y: 0 }); // クリック判定用
  const button = useRef<number>(0);

  const {
    orbitSpeed = 0.01,
    panSpeed = 0.01,
    zoomSpeed = 0.1,
    enablePicking = true,
  } = options;

  // ドラッグ判定のしきい値（ピクセル）
  const DRAG_THRESHOLD = 3;

  // マウスダウン
  const handleMouseDown = useCallback((e: MouseEvent) => {
    // 左クリック(0)、右クリック(2)、中クリック(1)
    if (e.button === 0 || e.button === 2 || e.button === 1) {
      isDragging.current = true;
      hasDragged.current = false;
      button.current = e.button;
      lastPos.current = { x: e.clientX, y: e.clientY };
      mouseDownPos.current = { x: e.clientX, y: e.clientY };
      if (e.button !== 0) {
        e.preventDefault();
      }
    }
  }, []);

  // マウス移動
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging.current || !engine) return;

    const deltaX = e.clientX - lastPos.current.x;
    const deltaY = e.clientY - lastPos.current.y;
    lastPos.current = { x: e.clientX, y: e.clientY };

    // ドラッグしたかどうか判定
    const distFromStart = Math.sqrt(
      Math.pow(e.clientX - mouseDownPos.current.x, 2) +
      Math.pow(e.clientY - mouseDownPos.current.y, 2)
    );
    if (distFromStart > DRAG_THRESHOLD) {
      hasDragged.current = true;
    }

    // 左ボタンはカメラ操作しない（選択用）
    if (button.current === 0) return;

    // 中ボタン または Shift+右ボタンでPan
    const isPan = button.current === 1 || e.shiftKey;

    if (isPan) {
      engine.panCamera(-deltaX * panSpeed, deltaY * panSpeed);
    } else {
      engine.orbitCamera(-deltaX * orbitSpeed, -deltaY * orbitSpeed);
    }
  }, [engine, orbitSpeed, panSpeed, DRAG_THRESHOLD]);

  // マウスアップ
  const handleMouseUp = useCallback((e: MouseEvent) => {
    // 左クリックでドラッグしていなかったらEntity選択
    if (
      enablePicking &&
      engine &&
      button.current === 0 &&
      !hasDragged.current
    ) {
      const canvas = canvasRef.current;
      if (canvas) {
        const rect = canvas.getBoundingClientRect();
        const canvasX = e.clientX - rect.left;
        const canvasY = e.clientY - rect.top;
        const pickedId = engine.pickEntityAtCanvas(canvasX, canvasY);

        if (pickedId !== null) {
          // Ctrl/Cmd押下で複数選択、そうでなければ単一選択
          selectEntity(pickedId, e.ctrlKey || e.metaKey);
        } else {
          // 何もないところをクリックで選択解除
          deselectAll();
        }
      }
    }
    isDragging.current = false;
    hasDragged.current = false;
  }, [engine, enablePicking, canvasRef, selectEntity, deselectAll]);

  // ホイール（ズーム）
  const handleWheel = useCallback((e: WheelEvent) => {
    if (!engine) return;
    e.preventDefault();
    // deltaYが正（下スクロール）でズームアウト、負（上スクロール）でズームイン
    const delta = e.deltaY > 0 ? -zoomSpeed : zoomSpeed;
    engine.zoomCamera(delta);
  }, [engine, zoomSpeed]);

  // 右クリックメニュー無効化
  const handleContextMenu = useCallback((e: MouseEvent) => {
    e.preventDefault();
  }, []);

  // キーボード操作
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (!engine) return;

    // Fキーで選択Entityにフォーカス
    if (e.key === 'f' || e.key === 'F') {
      if (selectedEntityIds.length > 0) {
        engine.focusOnEntity(selectedEntityIds[0]);
      }
    }
  }, [engine, selectedEntityIds]);

  // イベントリスナー登録
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    canvas.addEventListener('mousedown', handleMouseDown);
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('mouseup', handleMouseUp);
    canvas.addEventListener('wheel', handleWheel, { passive: false });
    canvas.addEventListener('contextmenu', handleContextMenu);
    canvas.addEventListener('keydown', handleKeyDown);

    const handleMouseLeave = () => {
      isDragging.current = false;
      hasDragged.current = false;
    };
    canvas.addEventListener('mouseleave', handleMouseLeave);

    return () => {
      canvas.removeEventListener('mousedown', handleMouseDown);
      canvas.removeEventListener('mousemove', handleMouseMove);
      canvas.removeEventListener('mouseup', handleMouseUp);
      canvas.removeEventListener('mouseleave', handleMouseLeave);
      canvas.removeEventListener('wheel', handleWheel);
      canvas.removeEventListener('contextmenu', handleContextMenu);
      canvas.removeEventListener('keydown', handleKeyDown);
    };
  }, [
    canvasRef,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
    handleWheel,
    handleContextMenu,
    handleKeyDown,
  ]);
}
