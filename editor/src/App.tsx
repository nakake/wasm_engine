import { useEffect, useRef, useState, useCallback } from 'react';
import init from './wasm/engine_wasm';
import { EngineAPI } from './engine';
import { EngineProvider } from './engine/context';
import { Toolbar } from './components/layout';
import { SceneHierarchy } from './components/hierarchy';
import { Inspector } from './components/inspector';
import { useViewportControls, useGizmo, useGizmoInteraction } from './components/viewport';
import './styles/editor.css';

// デバッグ用にグローバル公開
declare global {
  interface Window {
    engine: EngineAPI;
  }
}

function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const engineRef = useRef<EngineAPI | null>(null);
  const [engine, setEngine] = useState<EngineAPI | null>(null);
  const [wasmReady, setWasmReady] = useState(false);
  const isInitialized = useRef(false);

  // WASM初期化
  useEffect(() => {
    if (isInitialized.current) return;
    isInitialized.current = true;

    init().then(() => {
      console.log('WASM initialized');
      setWasmReady(true);
    }).catch((error) => {
      console.error('Failed to initialize WASM:', error);
    });
  }, []);

  // Canvas準備完了時にEngine初期化
  const initializeEngine = useCallback(async (canvas: HTMLCanvasElement) => {
    if (engineRef.current || !wasmReady) return;

    try {
      console.log('Initializing engine with canvas:', {
        width: canvas.width,
        height: canvas.height,
        clientWidth: canvas.clientWidth,
        clientHeight: canvas.clientHeight,
      });

      const eng = new EngineAPI();
      await eng.initialize(canvas);

      // 初期サイズを設定（キャンバスサイズと同期）
      if (canvas.width > 0 && canvas.height > 0) {
        eng.resize(canvas.width, canvas.height);
        console.log('Engine resized to:', canvas.width, canvas.height);
      }

      engineRef.current = eng;
      setEngine(eng);

      // デバッグ用にグローバル公開
      window.engine = eng;

      // 初期Entityを作成
      const cube1 = eng.createEntity('Cube1');
      const cube2 = eng.createEntity('Cube2');
      const cube3 = eng.createEntity('Cube3');

      eng.setPosition(cube1, { x: -2, y: 0, z: 0 });
      eng.setPosition(cube2, { x: 0, y: 0, z: 0 });
      eng.setPosition(cube3, { x: 2, y: 0, z: 0 });

      console.log('Created 3 cubes:', { cube1, cube2, cube3 });

      // レンダーループ開始
      console.log('Starting render loop');
      let lastTime = performance.now();
      let frameCount = 0;
      const render = (currentTime: number) => {
        const deltaTime = (currentTime - lastTime) / 1000;
        lastTime = currentTime;

        if (engineRef.current) {
          frameCount++;
          if (frameCount <= 3) {
            console.log(`Render frame ${frameCount}`);
          }
          engineRef.current.tick(deltaTime);
          requestAnimationFrame(render);
        }
      };
      requestAnimationFrame(render);
    } catch (error) {
      console.error('Failed to initialize engine:', error);
    }
  }, [wasmReady]);

  // クリーンアップ
  useEffect(() => {
    return () => {
      if (engineRef.current) {
        engineRef.current.dispose();
        engineRef.current = null;
      }
    };
  }, []);

  // WASM未準備
  if (!wasmReady) {
    return (
      <div className="editor-layout" style={{ justifyContent: 'center', alignItems: 'center' }}>
        <div style={{ color: '#888' }}>Loading WASM...</div>
      </div>
    );
  }

  // 常に同じ構造を返す（Canvasの再作成を防ぐ）
  return (
    <EngineProvider engine={engine}>
      <div className="editor-layout">
        <header className="editor-toolbar">
          {engine && <Toolbar />}
        </header>
        <main className="editor-main">
          <aside className="editor-panel editor-panel-left">
            {engine ? <SceneHierarchy /> : <div style={{ padding: 16, color: '#666' }}>Loading...</div>}
          </aside>
          <section className="editor-viewport">
            <ViewportCanvas
              canvasRef={canvasRef}
              onCanvasReady={initializeEngine}
              engine={engine}
            />
          </section>
          <aside className="editor-panel editor-panel-right">
            {engine ? <Inspector /> : <div style={{ padding: 16, color: '#666' }}>Loading...</div>}
          </aside>
        </main>
      </div>
    </EngineProvider>
  );
}

// Viewport内のCanvas（Engine初期化前から表示）
interface ViewportCanvasProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  onCanvasReady: (canvas: HTMLCanvasElement) => void;
  engine: EngineAPI | null;
}

function ViewportCanvas({ canvasRef, onCanvasReady, engine }: ViewportCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const initializedRef = useRef(false);

  // Canvas準備とEngine初期化 - ResizeObserverで確実にサイズを取得
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const handleResize = () => {
      const rect = container.getBoundingClientRect();
      const width = Math.floor(rect.width);
      const height = Math.floor(rect.height);

      console.log('ViewportCanvas resize:', { width, height, initialized: initializedRef.current });

      if (width > 0 && height > 0) {
        canvas.width = width;
        canvas.height = height;
        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;

        if (!initializedRef.current) {
          initializedRef.current = true;
          onCanvasReady(canvas);
        } else if (engine) {
          engine.resize(width, height);
        }
      }
    };

    const resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(container);

    // 初回チェック
    handleResize();

    return () => resizeObserver.disconnect();
  }, [canvasRef, onCanvasReady, engine]);

  // カメラ操作を有効化
  useViewportControls(canvasRef);

  // Gizmo同期を有効化
  useGizmo();

  // Gizmoインタラクション（ドラッグ操作）を有効化
  useGizmoInteraction(canvasRef);

  return (
    <div
      ref={containerRef}
      style={{ width: '100%', height: '100%', position: 'relative' }}
    >
      <canvas
        ref={canvasRef}
        style={{ display: 'block', position: 'absolute', top: 0, left: 0 }}
        tabIndex={0}
      />
    </div>
  );
}

// EngineProvider付きラッパー
function AppWithProvider() {
  return <App />;
}

export default AppWithProvider;
