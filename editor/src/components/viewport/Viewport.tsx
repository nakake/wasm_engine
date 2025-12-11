import { useRef, useEffect, useCallback } from 'react';
import { useEngine } from '../../engine/context';

interface ViewportProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
}

export function Viewport({ canvasRef }: ViewportProps) {
  const engine = useEngine();
  const containerRef = useRef<HTMLDivElement>(null);
  const initializedRef = useRef(false);

  // サイズ更新関数
  const updateSize = useCallback(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;

    const rect = container.getBoundingClientRect();
    const width = Math.floor(rect.width);
    const height = Math.floor(rect.height);

    if (width > 0 && height > 0) {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = width * dpr;
      canvas.height = height * dpr;
      canvas.style.width = `${width}px`;
      canvas.style.height = `${height}px`;
      engine.resize(width, height);
    }
  }, [engine, canvasRef]);

  // Canvasをコンテナ内に配置
  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas || initializedRef.current) return;

    // Canvasをコンテナ内に移動
    container.appendChild(canvas);
    canvas.style.display = 'block';
    canvas.style.position = 'absolute';
    canvas.style.top = '0';
    canvas.style.left = '0';
    initializedRef.current = true;

    // 初期サイズを設定
    // requestAnimationFrameで次フレームまで待つ
    requestAnimationFrame(() => {
      updateSize();
    });
  }, [canvasRef, updateSize]);

  // リサイズ監視
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const resizeObserver = new ResizeObserver(() => {
      updateSize();
    });

    resizeObserver.observe(container);
    return () => resizeObserver.disconnect();
  }, [updateSize]);

  return (
    <div
      ref={containerRef}
      style={{
        width: '100%',
        height: '100%',
        position: 'relative',
        overflow: 'hidden',
      }}
    />
  );
}
