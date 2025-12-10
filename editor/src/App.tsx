import { useEffect, useRef } from 'react';
import init from './wasm/engine_wasm';
import { EngineAPI, Vec3Helper } from './engine';

// デバッグ用にグローバル公開
declare global {
  interface Window {
    engine: EngineAPI;
  }
}

function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const engineRef = useRef<EngineAPI | null>(null);
  const isInitialized = useRef(false);

  useEffect(() => {
    if (isInitialized.current) return;
    isInitialized.current = true;

    let animationId: number;
    let lastTime = performance.now();

    const initEngine = async () => {
      try {
        // WASM初期化
        await init();
        console.log('WASM initialized');

        const canvas = canvasRef.current;
        if (!canvas) return;

        // EngineAPI作成
        const engine = new EngineAPI();
        await engine.initialize(canvas);
        engineRef.current = engine;

        // デバッグ用にグローバル公開
        window.engine = engine;

        // 3つのCubeを作成
        const cube1 = engine.createEntity('Cube1');
        const cube2 = engine.createEntity('Cube2');
        const cube3 = engine.createEntity('Cube3');

        // 異なる位置に配置
        engine.setPosition(cube1, Vec3Helper.create(-2, 0, 0));
        engine.setPosition(cube2, Vec3Helper.create(0, 0, 0));
        engine.setPosition(cube3, Vec3Helper.create(2, 0, 0));

        console.log('Created 3 cubes:', { cube1, cube2, cube3 });
        console.log('Entity count:', engine.getEntityCount());

        // レンダーループ
        const render = (currentTime: number) => {
          const deltaTime = (currentTime - lastTime) / 1000;
          lastTime = currentTime;

          if (engineRef.current) {
            engineRef.current.tick(deltaTime);
            animationId = requestAnimationFrame(render);
          }
        };
        animationId = requestAnimationFrame(render);
      } catch (error) {
        console.error('Failed to initialize engine:', error);
      }
    };

    initEngine();

    return () => {
      if (animationId) {
        cancelAnimationFrame(animationId);
      }
      if (engineRef.current) {
        engineRef.current.dispose();
        engineRef.current = null;
      }
    };
  }, []);

  return (
    <div style={{ width: '100vw', height: '100vh', margin: 0, padding: 0 }}>
      <canvas
        ref={canvasRef}
        style={{ width: '100%', height: '100%', display: 'block' }}
      />
    </div>
  );
}

export default App;
