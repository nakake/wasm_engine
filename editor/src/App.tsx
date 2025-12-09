import { useEffect, useRef } from 'react';
import init, { greet, Renderer } from './wasm/engine_wasm';

function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<Renderer | null>(null);
  const isInitialized = useRef(false);

  useEffect(() => {
    if (isInitialized.current) return;
    isInitialized.current = true;

    let animationId: number;
    let renderer: Renderer | null = null;

    const initEngine = async () => {
      try {
        // WASM初期化
        await init();
        console.log(greet('WebGPU'));

        const canvas = canvasRef.current;
        if (!canvas) return;

        // Renderer作成
        renderer = await Renderer.create(canvas);
        rendererRef.current = renderer;

        // レンダーループ
        const render = () => {
          if (renderer) {
            renderer.render();
            animationId = requestAnimationFrame(render);
          }
        };
        render();
      } catch (error) {
        console.error('Failed to initialize engine:', error);
      }
    };

    initEngine();

    return () => {
      if (animationId) {
        cancelAnimationFrame(animationId);
      }
      renderer = null;
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
