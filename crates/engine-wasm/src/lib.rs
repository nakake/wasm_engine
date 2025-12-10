// crates/engine-wasm/src/lib.rs

use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;
use web_sys::HtmlCanvasElement;

use engine_core::{World, EntityId, Transform, Name};
use glam::{Vec3, Quat};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// パニック時のスタックトレース表示
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

// 頂点データ構造
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// 三角形の頂点データ
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    }, // 上（赤）
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    }, // 左下（緑）
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    }, // 右下（青）
];

// シェーダーコード（WGSL）
const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

// Renderer構造体
#[wasm_bindgen]
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
}

#[wasm_bindgen]
impl Renderer {
    /// 新しいRendererを作成（非同期）
    pub async fn create(canvas: HtmlCanvasElement) -> Result<Renderer, JsValue> {
        console_log!("Initializing WebGPU...");

        // Canvas サイズ取得
        let width = canvas.client_width() as u32;
        let height = canvas.client_height() as u32;
        console_log!("Canvas size: {}x{}", width, height);

        // wgpu インスタンス作成
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Surface 作成（wasm32ターゲット用）
        #[cfg(target_arch = "wasm32")]
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {:?}", e)))?;

        #[cfg(not(target_arch = "wasm32"))]
        let surface: wgpu::Surface<'static> = unreachable!("This code is only for wasm32 target");

        // Adapter 取得
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to find suitable adapter: {:?}", e)))?;

        console_log!("Adapter: {:?}", adapter.get_info());

        // Device & Queue 作成
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: Default::default(),
                    experimental_features: Default::default(),
                    trace: Default::default(),
                },
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create device: {:?}", e)))?;

        console_log!("Device created successfully");

        // Surface 設定
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // シェーダーモジュール作成
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        // Render Pipeline 作成
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // 頂点バッファ作成
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        console_log!("Renderer initialized successfully");

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size: (width, height),
            render_pipeline,
            vertex_buffer,
            num_vertices: VERTICES.len() as u32,
        })
    }

    /// レンダリング実行
    pub fn render(&self) -> Result<(), JsValue> {
        // 次のフレームを取得
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to get surface texture: {:?}", e)))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // コマンドエンコーダー作成
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // レンダーパス
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
        }

        // コマンド送信
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Canvas サイズ変更
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = (width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            console_log!("Resized to {}x{}", width, height);
        }
    }

    /// 現在のサイズ取得
    pub fn width(&self) -> u32 {
        self.size.0
    }

    pub fn height(&self) -> u32 {
        self.size.1
    }
}

// テスト用のgreet関数
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello Hello, {}!", name)
}

/// Engine構造体
/// WorldとRendererを統合し、JSから操作可能なAPIを提供
#[wasm_bindgen]
pub struct Engine {
    world: World,
    renderer: Renderer,
}

#[wasm_bindgen]
impl Engine {
    /// 新しいEngineを作成（非同期）
    pub async fn create(canvas: HtmlCanvasElement) -> Result<Engine, JsValue> {
        console_log!("Creating Engine...");
        let renderer = Renderer::create(canvas).await?;
        let world = World::new();
        console_log!("Engine created successfully");
        Ok(Self { world, renderer })
    }

    /// Entityを作成し、IDを返す
    pub fn create_entity(&mut self, name: &str) -> u32 {
        let entity = self.world.spawn();
        self.world.insert(entity, Name::new(name));
        self.world.insert(entity, Transform::identity());
        console_log!("Created entity: {} (id: {})", name, entity.to_u32());
        entity.to_u32()
    }

    /// Entityを削除
    pub fn delete_entity(&mut self, id: u32) -> bool {
        let entity = EntityId::from_u32(id);
        let result = self.world.despawn(entity);
        if result {
            console_log!("Deleted entity: {}", id);
        }
        result
    }

    /// 位置を設定
    pub fn set_position(&mut self, id: u32, x: f32, y: f32, z: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.position = Vec3::new(x, y, z);
        }
    }

    /// 回転を設定（クォータニオン）
    pub fn set_rotation(&mut self, id: u32, x: f32, y: f32, z: f32, w: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.rotation = Quat::from_xyzw(x, y, z, w);
        }
    }

    /// スケールを設定
    pub fn set_scale(&mut self, id: u32, x: f32, y: f32, z: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.scale = Vec3::new(x, y, z);
        }
    }

    /// 位置を取得（x, y, zの配列）
    pub fn get_position(&self, id: u32) -> Option<Vec<f32>> {
        let entity = EntityId::from_u32(id);
        self.world.get::<Transform>(entity).map(|t| vec![t.position.x, t.position.y, t.position.z])
    }

    /// 回転を取得（x, y, z, wの配列）
    pub fn get_rotation(&self, id: u32) -> Option<Vec<f32>> {
        let entity = EntityId::from_u32(id);
        self.world.get::<Transform>(entity).map(|t| vec![t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w])
    }

    /// スケールを取得（x, y, zの配列）
    pub fn get_scale(&self, id: u32) -> Option<Vec<f32>> {
        let entity = EntityId::from_u32(id);
        self.world.get::<Transform>(entity).map(|t| vec![t.scale.x, t.scale.y, t.scale.z])
    }

    /// Entity名を取得
    pub fn get_name(&self, id: u32) -> Option<String> {
        let entity = EntityId::from_u32(id);
        self.world.get::<Name>(entity).map(|n| n.as_str().to_string())
    }

    /// Entityが生存しているか確認
    pub fn is_alive(&self, id: u32) -> bool {
        let entity = EntityId::from_u32(id);
        self.world.is_alive(entity)
    }

    /// Entity数を取得
    pub fn entity_count(&self) -> usize {
        self.world.entity_count()
    }

    /// フレーム更新（レンダリング含む）
    pub fn tick(&mut self, _delta_time: f32) -> Result<(), JsValue> {
        // 将来: delta_timeを使ったシステム更新
        self.renderer.render()
    }

    /// Canvasリサイズ
    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    /// 幅取得
    pub fn width(&self) -> u32 {
        self.renderer.width()
    }

    /// 高さ取得
    pub fn height(&self) -> u32 {
        self.renderer.height()
    }
}
