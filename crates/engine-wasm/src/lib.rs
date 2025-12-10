// crates/engine-wasm/src/lib.rs

use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;
use web_sys::HtmlCanvasElement;

use engine_core::{EntityId, ModelUniform, Name, Transform, World};
use engine_renderer::{Camera, Mesh, Vertex};
use glam::{Quat, Vec3};

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

// 3D描画用シェーダーコード（WGSL）
const SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
}

struct ModelUniform {
    model: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * model.model * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    // Transform normal to world space (simplified, assumes no non-uniform scale)
    out.normal = (model.model * vec4<f32>(in.normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let normal = normalize(in.normal);
    let diffuse = max(dot(normal, light_dir), 0.3);
    return vec4<f32>(in.color * diffuse, 1.0);
}
"#;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Depth Texture作成
fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    (texture, view)
}

// Renderer構造体
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
    render_pipeline: wgpu::RenderPipeline,

    // Camera
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    // Model (per-entity transform)
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,

    // Cube mesh
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    // Depth buffer
    #[allow(dead_code)]
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

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
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
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

        // Depth Texture 作成
        let (depth_texture, depth_view) = create_depth_texture(&device, width, height);

        // Camera 作成
        let camera = Camera::new(width as f32 / height as f32);
        let camera_uniform = camera.uniform();

        // Camera Uniform Buffer
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Camera Bind Group Layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Camera Bind Group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Model Uniform Buffer (for per-entity transform)
        let model_uniform = ModelUniform::identity();
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Buffer"),
            contents: bytemuck::bytes_of(&model_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Model Bind Group Layout
        let model_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Model Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Model Bind Group
        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Model Bind Group"),
            layout: &model_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        });

        // Cube Mesh 作成
        let cube = Mesh::cube();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Vertex Buffer"),
            contents: bytemuck::cast_slice(&cube.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Index Buffer"),
            contents: bytemuck::cast_slice(&cube.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = cube.index_count() as u32;

        // シェーダーモジュール作成
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        // Render Pipeline Layout (with bind groups)
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &model_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Render Pipeline
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        console_log!("Renderer initialized successfully");

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size: (width, height),
            render_pipeline,
            camera,
            camera_buffer,
            camera_bind_group,
            model_buffer,
            model_bind_group,
            vertex_buffer,
            index_buffer,
            num_indices,
            depth_texture,
            depth_view,
        })
    }

    /// 単一のCubeをレンダリング（固定位置）
    pub fn render(&self) -> Result<(), JsValue> {
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to get surface texture: {:?}", e)))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Camera uniform更新
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera.uniform()),
        );

        // Default model (identity)
        let model_uniform = ModelUniform::identity();
        self.queue
            .write_buffer(&self.model_buffer, 0, bytemuck::bytes_of(&model_uniform));

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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.model_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

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

            // Depth Texture再作成
            let (depth_texture, depth_view) = create_depth_texture(&self.device, width, height);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;

            // Camera aspect更新
            self.camera.set_aspect(width as f32 / height as f32);

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

    /// Queue参照を取得
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Model Buffer参照を取得
    pub fn model_buffer(&self) -> &wgpu::Buffer {
        &self.model_buffer
    }

    /// Worldの全Transformを持つEntityをレンダリング
    pub fn render_world(&self, world: &World) -> Result<(), JsValue> {
        // 先に全Transformを収集
        let transforms: Vec<ModelUniform> = world
            .iter_with::<Transform>()
            .map(|(_, t)| ModelUniform::from_transform(t))
            .collect();

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to get surface texture: {:?}", e)))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Camera uniform更新
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera.uniform()),
        );

        // 各Entityを個別のコマンドで描画
        let mut commands = Vec::new();

        for (i, model_uniform) in transforms.iter().enumerate() {
            // Model uniform更新
            self.queue
                .write_buffer(&self.model_buffer, 0, bytemuck::bytes_of(model_uniform));

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(&format!("Render Encoder {}", i)),
                });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("Render Pass {}", i)),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            // 最初のパスのみClear、それ以降はLoad
                            load: if i == 0 {
                                wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                })
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: if i == 0 {
                                wgpu::LoadOp::Clear(1.0)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }

            commands.push(encoder.finish());
            // 各コマンドを個別にsubmitしてバッファ更新を反映
            self.queue.submit(commands.drain(..));
        }

        // Entityが0の場合は背景のみ描画
        if transforms.is_empty() {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Clear Encoder"),
                });

            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Clear Pass"),
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
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
            }

            self.queue.submit(std::iter::once(encoder.finish()));
        }

        output.present();

        Ok(())
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
        self.world
            .get::<Transform>(entity)
            .map(|t| vec![t.position.x, t.position.y, t.position.z])
    }

    /// 回転を取得（x, y, z, wの配列）
    pub fn get_rotation(&self, id: u32) -> Option<Vec<f32>> {
        let entity = EntityId::from_u32(id);
        self.world.get::<Transform>(entity).map(|t| {
            vec![
                t.rotation.x,
                t.rotation.y,
                t.rotation.z,
                t.rotation.w,
            ]
        })
    }

    /// スケールを取得（x, y, zの配列）
    pub fn get_scale(&self, id: u32) -> Option<Vec<f32>> {
        let entity = EntityId::from_u32(id);
        self.world
            .get::<Transform>(entity)
            .map(|t| vec![t.scale.x, t.scale.y, t.scale.z])
    }

    /// Entity名を取得
    pub fn get_name(&self, id: u32) -> Option<String> {
        let entity = EntityId::from_u32(id);
        self.world
            .get::<Name>(entity)
            .map(|n| n.as_str().to_string())
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
        self.renderer.render_world(&self.world)
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
