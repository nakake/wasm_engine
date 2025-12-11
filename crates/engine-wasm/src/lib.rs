// crates/engine-wasm/src/lib.rs

use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;
use web_sys::HtmlCanvasElement;
use js_sys::Function;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use engine_core::{EntityId, ModelUniform, Name, QueryDescriptor, QueryResult, Transform, World};
use engine_renderer::{
    glam, Camera, Mesh, Vertex, Ray,
    GizmoMode, GizmoAxis, GizmoState, GizmoVertex,
    create_arrow_vertices, create_plane_vertices, create_circle_vertices,
    create_scale_axis_vertices, create_center_box_vertices,
};
use glam::{Quat, Vec3, Mat4};

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

// Gizmo描画用シェーダーコード（WGSL）
const GIZMO_SHADER: &str = r#"
struct GizmoUniform {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: GizmoUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * uniforms.model * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// Gizmo用Uniform構造体
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GizmoUniform {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

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

    // Gizmo
    gizmo_state: GizmoState,
    gizmo_pipeline: wgpu::RenderPipeline,
    gizmo_uniform_buffer: wgpu::Buffer,
    gizmo_bind_group: wgpu::BindGroup,
}

impl Renderer {
    /// 新しいRendererを作成（非同期）
    pub async fn create(canvas: HtmlCanvasElement) -> Result<Renderer, JsValue> {
        console_log!("Initializing WebGPU...");

        // Canvas サイズ取得（canvas属性のwidth/heightを使用）
        let width = canvas.width();
        let height = canvas.height();
        console_log!("Canvas size: {}x{}", width, height);

        // サイズが0の場合はエラー
        if width == 0 || height == 0 {
            return Err(JsValue::from_str(&format!(
                "Canvas size is invalid: {}x{}. Set canvas.width/height before initializing.",
                width, height
            )));
        }

        // wgpu インスタンス作成（WebGPUバックエンドを使用）
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
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

        // ブラウザ推奨フォーマット（bgra8unorm）を優先、なければ最初のフォーマット
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| **f == wgpu::TextureFormat::Bgra8Unorm)
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // alpha_modeを選択（Opaqueを優先）
        let alpha_mode = if surface_caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            surface_caps.alpha_modes[0]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
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

        // ========================================
        // Gizmoパイプライン作成
        // ========================================

        // Gizmo Uniform Buffer
        let gizmo_uniform = GizmoUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            model: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let gizmo_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Uniform Buffer"),
            contents: bytemuck::bytes_of(&gizmo_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Gizmo Bind Group Layout
        let gizmo_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Gizmo Bind Group Layout"),
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

        // Gizmo Bind Group
        let gizmo_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gizmo Bind Group"),
            layout: &gizmo_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: gizmo_uniform_buffer.as_entire_binding(),
            }],
        });

        // Gizmo Shader
        let gizmo_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gizmo Shader"),
            source: wgpu::ShaderSource::Wgsl(GIZMO_SHADER.into()),
        });

        // Gizmo Pipeline Layout
        let gizmo_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Gizmo Pipeline Layout"),
                bind_group_layouts: &[&gizmo_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Gizmo Render Pipeline（深度テスト無効で常に手前に描画）
        let gizmo_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gizmo Render Pipeline"),
            layout: Some(&gizmo_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &gizmo_shader,
                entry_point: Some("vs_main"),
                buffers: &[GizmoVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &gizmo_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // 両面描画
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None, // 深度テスト無効（常に手前）
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let gizmo_state = GizmoState::default();

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
            gizmo_state,
            gizmo_pipeline,
            gizmo_uniform_buffer,
            gizmo_bind_group,
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

    // ========================================================================
    // カメラ操作
    // ========================================================================

    /// カメラをターゲット周りで回転
    pub fn orbit_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.orbit(delta_x, delta_y);
    }

    /// カメラを平行移動
    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan(delta_x, delta_y);
    }

    /// カメラをズーム
    pub fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    /// カメラターゲットを設定
    pub fn set_camera_target(&mut self, target: Vec3) {
        self.camera.set_target(target);
    }

    /// カメラ位置を取得
    pub fn camera_position(&self) -> Vec3 {
        self.camera.position()
    }

    /// カメラターゲットを取得
    pub fn camera_target(&self) -> Vec3 {
        self.camera.target()
    }

    /// スクリーン座標からワールド空間のレイを生成
    pub fn screen_to_ray(&self, screen_x: f32, screen_y: f32) -> engine_renderer::Ray {
        self.camera.screen_to_ray(screen_x, screen_y)
    }

    /// スクリーン座標をワールド座標に変換
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32, depth: f32) -> Vec3 {
        self.camera.screen_to_world(screen_x, screen_y, depth)
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

        // Gizmo描画
        if self.gizmo_state.visible {
            self.render_gizmo(&view)?;
        }

        output.present();

        Ok(())
    }

    /// Gizmoを描画
    fn render_gizmo(&self, view: &wgpu::TextureView) -> Result<(), JsValue> {
        // Gizmoの頂点を生成
        let vertices = self.build_gizmo_vertices();
        if vertices.is_empty() {
            return Ok(());
        }

        // 一時的な頂点バッファを作成
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Uniform更新
        let model = self.gizmo_state.model_matrix(self.camera.position());
        let gizmo_uniform = GizmoUniform {
            view_proj: self.camera.build_view_projection_matrix().to_cols_array_2d(),
            model: model.to_cols_array_2d(),
        };
        self.queue.write_buffer(
            &self.gizmo_uniform_buffer,
            0,
            bytemuck::bytes_of(&gizmo_uniform),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Gizmo Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gizmo Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // 既存の描画を保持
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None, // 深度テストなし
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.gizmo_pipeline);
            render_pass.set_bind_group(0, &self.gizmo_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..vertices.len() as u32, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    /// 現在のモードに応じたGizmo頂点を生成
    fn build_gizmo_vertices(&self) -> Vec<GizmoVertex> {
        let mut vertices = Vec::new();

        match self.gizmo_state.mode {
            GizmoMode::Translate => {
                // 3軸の矢印
                vertices.extend(create_arrow_vertices(GizmoAxis::X, self.gizmo_state.axis_color(GizmoAxis::X)));
                vertices.extend(create_arrow_vertices(GizmoAxis::Y, self.gizmo_state.axis_color(GizmoAxis::Y)));
                vertices.extend(create_arrow_vertices(GizmoAxis::Z, self.gizmo_state.axis_color(GizmoAxis::Z)));
                // 平面ハンドル
                vertices.extend(create_plane_vertices(GizmoAxis::XY, self.gizmo_state.axis_color(GizmoAxis::XY)));
                vertices.extend(create_plane_vertices(GizmoAxis::YZ, self.gizmo_state.axis_color(GizmoAxis::YZ)));
                vertices.extend(create_plane_vertices(GizmoAxis::XZ, self.gizmo_state.axis_color(GizmoAxis::XZ)));
            }
            GizmoMode::Rotate => {
                // 3軸の円
                vertices.extend(create_circle_vertices(GizmoAxis::X, self.gizmo_state.axis_color(GizmoAxis::X)));
                vertices.extend(create_circle_vertices(GizmoAxis::Y, self.gizmo_state.axis_color(GizmoAxis::Y)));
                vertices.extend(create_circle_vertices(GizmoAxis::Z, self.gizmo_state.axis_color(GizmoAxis::Z)));
            }
            GizmoMode::Scale => {
                // 3軸のスケールハンドル
                vertices.extend(create_scale_axis_vertices(GizmoAxis::X, self.gizmo_state.axis_color(GizmoAxis::X)));
                vertices.extend(create_scale_axis_vertices(GizmoAxis::Y, self.gizmo_state.axis_color(GizmoAxis::Y)));
                vertices.extend(create_scale_axis_vertices(GizmoAxis::Z, self.gizmo_state.axis_color(GizmoAxis::Z)));
                // 中心ボックス
                vertices.extend(create_center_box_vertices(self.gizmo_state.axis_color(GizmoAxis::All)));
            }
        }

        vertices
    }

    // ========================================================================
    // Gizmo API
    // ========================================================================

    /// Gizmoモードを設定
    pub fn set_gizmo_mode(&mut self, mode: &str) {
        self.gizmo_state.mode = match mode {
            "translate" => GizmoMode::Translate,
            "rotate" => GizmoMode::Rotate,
            "scale" => GizmoMode::Scale,
            _ => return,
        };
    }

    /// Gizmo表示/非表示を設定
    pub fn set_gizmo_visible(&mut self, visible: bool) {
        self.gizmo_state.visible = visible;
    }

    /// Gizmo位置を設定
    pub fn set_gizmo_position(&mut self, x: f32, y: f32, z: f32) {
        self.gizmo_state.position = Vec3::new(x, y, z);
    }

    /// Gizmo回転を設定（Local Space用）
    pub fn set_gizmo_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) {
        self.gizmo_state.rotation = Quat::from_xyzw(x, y, z, w);
    }

    /// ホバー中の軸を設定
    pub fn set_gizmo_hovered_axis(&mut self, axis: &str) {
        self.gizmo_state.hovered_axis = axis.parse().unwrap_or_default();
    }

    /// アクティブ（操作中）の軸を設定
    pub fn set_gizmo_active_axis(&mut self, axis: &str) {
        self.gizmo_state.active_axis = axis.parse().unwrap_or_default();
    }

    /// Gizmo表示状態を取得
    pub fn is_gizmo_visible(&self) -> bool {
        self.gizmo_state.visible
    }
}

// テスト用のgreet関数
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello Hello, {}!", name)
}

/// クエリ結果のハッシュを計算
fn calculate_hash(result: &QueryResult) -> u64 {
    let json = serde_json::to_string(result).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    hasher.finish()
}

/// クエリ購読情報
struct QuerySubscription {
    query: QueryDescriptor,
    callback: Function,
    last_result_hash: u64,
}

/// 購読マネージャー
struct QuerySubscriptionManager {
    subscriptions: HashMap<u32, QuerySubscription>,
    next_id: u32,
}

impl QuerySubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn subscribe(&mut self, query: QueryDescriptor, callback: Function) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.subscriptions.insert(
            id,
            QuerySubscription {
                query,
                callback,
                last_result_hash: 0,
            },
        );

        id
    }

    pub fn unsubscribe(&mut self, id: u32) -> bool {
        self.subscriptions.remove(&id).is_some()
    }
}

/// Engine構造体
/// WorldとRendererを統合し、JSから操作可能なAPIを提供
#[wasm_bindgen]
pub struct Engine {
    world: World,
    renderer: Renderer,
    subscriptions: QuerySubscriptionManager,
    /// Gizmoドラッグ開始時のレイ
    gizmo_drag_ray: Option<Ray>,
    /// Gizmoドラッグ中の軸
    gizmo_drag_axis: GizmoAxis,
}

#[wasm_bindgen]
impl Engine {
    /// 新しいEngineを作成（非同期）
    pub async fn create(canvas: HtmlCanvasElement) -> Result<Engine, JsValue> {
        console_log!("Creating Engine...");
        let renderer = Renderer::create(canvas).await?;
        let world = World::new();
        let subscriptions = QuerySubscriptionManager::new();
        console_log!("Engine created successfully");
        Ok(Self {
            world,
            renderer,
            subscriptions,
            gizmo_drag_ray: None,
            gizmo_drag_axis: GizmoAxis::None,
        })
    }

    /// Entityを作成し、IDを返す
    pub fn create_entity(&mut self, name: &str) -> u32 {
        let entity = self.world.spawn();
        self.world.insert(entity, Name::new(name));
        self.world.insert(entity, Transform::identity());
        console_log!("Created entity: {} (id: {})", name, entity.to_u32());
        self.check_subscriptions();
        entity.to_u32()
    }

    /// Entityを削除
    pub fn delete_entity(&mut self, id: u32) -> bool {
        let entity = EntityId::from_u32(id);
        let result = self.world.despawn(entity);
        if result {
            console_log!("Deleted entity: {}", id);
            self.check_subscriptions();
        }
        result
    }

    /// 位置を設定
    pub fn set_position(&mut self, id: u32, x: f32, y: f32, z: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.position = Vec3::new(x, y, z);
            self.check_subscriptions();
        }
    }

    /// 回転を設定（クォータニオン）
    pub fn set_rotation(&mut self, id: u32, x: f32, y: f32, z: f32, w: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.rotation = Quat::from_xyzw(x, y, z, w);
            self.check_subscriptions();
        }
    }

    /// スケールを設定
    pub fn set_scale(&mut self, id: u32, x: f32, y: f32, z: f32) {
        let entity = EntityId::from_u32(id);
        if let Some(transform) = self.world.get_mut::<Transform>(entity) {
            transform.scale = Vec3::new(x, y, z);
            self.check_subscriptions();
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

    /// Entity名を設定
    pub fn set_name(&mut self, id: u32, name: &str) {
        let entity = EntityId::from_u32(id);
        if let Some(n) = self.world.get_mut::<Name>(entity) {
            *n = Name::new(name);
            self.check_subscriptions();
        }
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

    /// クエリ実行
    ///
    /// # Arguments
    /// * `query_json` - QueryDescriptor の JSON文字列
    ///
    /// # Returns
    /// QueryResult の JsValue（JSON形式）
    pub fn execute_query(&self, query_json: &str) -> Result<JsValue, JsValue> {
        // JSONパース
        let query: QueryDescriptor = serde_json::from_str(query_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid query JSON: {}", e)))?;

        // クエリ実行
        let result = self.world.execute_query(&query);

        // JsValueに変換
        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// クエリを購読
    ///
    /// # Arguments
    /// * `query_json` - QueryDescriptor の JSON文字列
    /// * `callback` - 結果変更時に呼ばれる関数
    ///
    /// # Returns
    /// subscription_id
    pub fn subscribe_query(
        &mut self,
        query_json: &str,
        callback: Function,
    ) -> Result<u32, JsValue> {
        let query: QueryDescriptor = serde_json::from_str(query_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid query JSON: {}", e)))?;

        let id = self.subscriptions.subscribe(query, callback);

        // 初回実行
        self.notify_subscription(id);

        Ok(id)
    }

    /// 購読解除
    pub fn unsubscribe_query(&mut self, subscription_id: u32) -> bool {
        self.subscriptions.unsubscribe(subscription_id)
    }

    // ========================================================================
    // カメラ操作 API
    // ========================================================================

    /// カメラをターゲット周りで回転（Orbit）
    ///
    /// # Arguments
    /// * `delta_x` - 水平方向の回転量（ラジアン）
    /// * `delta_y` - 垂直方向の回転量（ラジアン）
    pub fn orbit_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.renderer.orbit_camera(delta_x, delta_y);
    }

    /// カメラを平行移動（Pan）
    ///
    /// # Arguments
    /// * `delta_x` - 右方向への移動量
    /// * `delta_y` - 上方向への移動量
    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.renderer.pan_camera(delta_x, delta_y);
    }

    /// カメラをズーム
    ///
    /// # Arguments
    /// * `delta` - 正で近づく、負で遠ざかる
    pub fn zoom_camera(&mut self, delta: f32) {
        self.renderer.zoom_camera(delta);
    }

    /// カメラターゲットを設定
    pub fn set_camera_target(&mut self, x: f32, y: f32, z: f32) {
        self.renderer.set_camera_target(Vec3::new(x, y, z));
    }

    /// カメラ位置を取得
    pub fn get_camera_position(&self) -> Vec<f32> {
        let pos = self.renderer.camera_position();
        vec![pos.x, pos.y, pos.z]
    }

    /// カメラターゲットを取得
    pub fn get_camera_target(&self) -> Vec<f32> {
        let target = self.renderer.camera_target();
        vec![target.x, target.y, target.z]
    }

    // ========================================================================
    // Entity Picking API
    // ========================================================================

    /// スクリーン座標からEntityを選択（レイキャスト）
    ///
    /// # Arguments
    /// * `screen_x` - スクリーンX座標 (0.0〜1.0)
    /// * `screen_y` - スクリーンY座標 (0.0〜1.0)
    ///
    /// # Returns
    /// * Entity ID (>= 0) if hit
    /// * -1 if no entity was hit
    pub fn pick_entity(&self, screen_x: f32, screen_y: f32) -> i32 {
        let ray = self.renderer.screen_to_ray(screen_x, screen_y);

        let mut closest: Option<(EntityId, f32)> = None;

        // 全Entityをチェック
        for entity_id in self.world.iter_entities() {
            if let Some(transform) = self.world.get::<Transform>(entity_id) {
                // 簡易Bounding Box (1x1x1 cube at transform position)
                let aabb = engine_renderer::AABB::unit_cube(transform.position, transform.scale);

                if let Some(t) = ray.intersect_aabb(&aabb) {
                    match closest {
                        None => closest = Some((entity_id, t)),
                        Some((_, prev_t)) if t < prev_t => closest = Some((entity_id, t)),
                        _ => {}
                    }
                }
            }
        }

        match closest {
            Some((id, _)) => id.to_u32() as i32,
            None => -1,
        }
    }

    /// スクリーン座標をワールド座標に変換
    ///
    /// # Arguments
    /// * `screen_x` - スクリーンX座標 (0.0〜1.0)
    /// * `screen_y` - スクリーンY座標 (0.0〜1.0)
    /// * `depth` - 深度値 (0.0 = near, 1.0 = far)
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32, depth: f32) -> Vec<f32> {
        let world_pos = self.renderer.screen_to_world(screen_x, screen_y, depth);
        vec![world_pos.x, world_pos.y, world_pos.z]
    }

    // ========================================================================
    // Gizmo API
    // ========================================================================

    /// Gizmoモードを設定
    /// @param mode - "translate" | "rotate" | "scale"
    pub fn set_gizmo_mode(&mut self, mode: &str) {
        self.renderer.set_gizmo_mode(mode);
    }

    /// Gizmo表示/非表示を設定
    pub fn set_gizmo_visible(&mut self, visible: bool) {
        self.renderer.set_gizmo_visible(visible);
    }

    /// Gizmo位置を設定
    pub fn set_gizmo_position(&mut self, x: f32, y: f32, z: f32) {
        self.renderer.set_gizmo_position(x, y, z);
    }

    /// Gizmo回転を設定（Local Space用）
    pub fn set_gizmo_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) {
        self.renderer.set_gizmo_rotation(x, y, z, w);
    }

    /// ホバー中の軸を設定
    /// @param axis - "x" | "y" | "z" | "xy" | "yz" | "xz" | "all" | "none"
    pub fn set_gizmo_hovered_axis(&mut self, axis: &str) {
        self.renderer.set_gizmo_hovered_axis(axis);
    }

    /// アクティブ（操作中）の軸を設定
    pub fn set_gizmo_active_axis(&mut self, axis: &str) {
        self.renderer.set_gizmo_active_axis(axis);
    }

    /// Gizmo表示状態を取得
    pub fn is_gizmo_visible(&self) -> bool {
        self.renderer.is_gizmo_visible()
    }

    /// Gizmoヒットテスト
    /// @param screen_x スクリーンX座標 (0.0〜1.0)
    /// @param screen_y スクリーンY座標 (0.0〜1.0)
    /// @returns ヒットした軸名 ("x", "y", "z", "xy", "yz", "xz", "all", "")
    pub fn gizmo_hit_test(&self, screen_x: f32, screen_y: f32) -> String {
        let ray = self.renderer.camera.screen_to_ray(screen_x, screen_y);
        let camera_pos = self.renderer.camera.position();
        let axis = self.renderer.gizmo_state.hit_test(&ray, camera_pos);
        axis_to_string(axis)
    }

    /// Gizmoドラッグ開始
    /// @param screen_x スクリーンX座標 (0.0〜1.0)
    /// @param screen_y スクリーンY座標 (0.0〜1.0)
    /// @returns ドラッグ開始した軸名（""の場合はヒットなし）
    pub fn start_gizmo_drag(&mut self, screen_x: f32, screen_y: f32) -> String {
        let ray = self.renderer.camera.screen_to_ray(screen_x, screen_y);
        let camera_pos = self.renderer.camera.position();
        let axis = self.renderer.gizmo_state.hit_test(&ray, camera_pos);

        if axis != GizmoAxis::None {
            self.gizmo_drag_ray = Some(ray);
            self.gizmo_drag_axis = axis;
            self.renderer.gizmo_state.active_axis = axis;
        }

        axis_to_string(axis)
    }

    /// Gizmoドラッグ更新（Translate/Scaleモード）
    /// @param screen_x スクリーンX座標 (0.0〜1.0)
    /// @param screen_y スクリーンY座標 (0.0〜1.0)
    /// @returns [dx, dy, dz] 移動/スケール変化量
    pub fn update_gizmo_drag(&mut self, screen_x: f32, screen_y: f32) -> Vec<f32> {
        if self.gizmo_drag_axis == GizmoAxis::None {
            return vec![0.0, 0.0, 0.0];
        }

        let ray = self.renderer.camera.screen_to_ray(screen_x, screen_y);
        let camera_pos = self.renderer.camera.position();

        let prev_ray = self.gizmo_drag_ray.unwrap_or(ray);

        let delta = match self.renderer.gizmo_state.mode {
            GizmoMode::Translate => {
                self.renderer.gizmo_state.calculate_translate_drag(
                    self.gizmo_drag_axis, &ray, &prev_ray, camera_pos
                )
            }
            GizmoMode::Scale => {
                self.renderer.gizmo_state.calculate_scale_drag(
                    self.gizmo_drag_axis, &ray, &prev_ray, camera_pos
                )
            }
            GizmoMode::Rotate => {
                // Rotateモードは update_gizmo_drag_rotate を使う
                Vec3::ZERO
            }
        };

        self.gizmo_drag_ray = Some(ray);

        vec![delta.x, delta.y, delta.z]
    }

    /// Gizmoドラッグ更新（Rotateモード）
    /// @param screen_x スクリーンX座標 (0.0〜1.0)
    /// @param screen_y スクリーンY座標 (0.0〜1.0)
    /// @returns [qx, qy, qz, qw] 回転差分（Quaternion）
    pub fn update_gizmo_drag_rotate(&mut self, screen_x: f32, screen_y: f32) -> Vec<f32> {
        if self.gizmo_drag_axis == GizmoAxis::None {
            return vec![0.0, 0.0, 0.0, 1.0];
        }

        let ray = self.renderer.camera.screen_to_ray(screen_x, screen_y);
        let prev_ray = self.gizmo_drag_ray.unwrap_or(ray);

        let rot = self.renderer.gizmo_state.calculate_rotate_drag(
            self.gizmo_drag_axis, &ray, &prev_ray
        );

        self.gizmo_drag_ray = Some(ray);

        vec![rot.x, rot.y, rot.z, rot.w]
    }

    /// Gizmoドラッグ終了
    pub fn end_gizmo_drag(&mut self) {
        self.gizmo_drag_ray = None;
        self.gizmo_drag_axis = GizmoAxis::None;
        self.renderer.gizmo_state.active_axis = GizmoAxis::None;
    }

    /// 全購読のチェック・通知
    fn check_subscriptions(&mut self) {
        let ids: Vec<u32> = self.subscriptions.subscriptions.keys().copied().collect();
        for id in ids {
            self.notify_subscription(id);
        }
    }

    /// 単一の購読を通知
    fn notify_subscription(&mut self, id: u32) {
        if let Some(sub) = self.subscriptions.subscriptions.get_mut(&id) {
            let result = self.world.execute_query(&sub.query);
            let hash = calculate_hash(&result);

            if hash != sub.last_result_hash {
                sub.last_result_hash = hash;

                // コールバック呼び出し
                if let Ok(js_result) = serde_wasm_bindgen::to_value(&result) {
                    let _ = sub.callback.call1(&JsValue::NULL, &js_result);
                }
            }
        }
    }
}

/// GizmoAxis を文字列に変換
fn axis_to_string(axis: GizmoAxis) -> String {
    match axis {
        GizmoAxis::X => "x".to_string(),
        GizmoAxis::Y => "y".to_string(),
        GizmoAxis::Z => "z".to_string(),
        GizmoAxis::XY => "xy".to_string(),
        GizmoAxis::YZ => "yz".to_string(),
        GizmoAxis::XZ => "xz".to_string(),
        GizmoAxis::All => "all".to_string(),
        GizmoAxis::None => "".to_string(),
    }
}
