//! Rendererモジュール
//!
//! WebGPUを使用したレンダリング機能を提供

mod depth;
mod gizmo_pipeline;
mod gpu_context;
mod scene_pipeline;

pub use gizmo_pipeline::GizmoUniform;
pub use gpu_context::GpuContext;

use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;
use web_sys::HtmlCanvasElement;

use crate::utils::console_log;

use engine_core::{ModelUniform, Transform, World};
use engine_renderer::{
    glam, Camera, GizmoAxis, GizmoMode, GizmoState, GizmoVertex,
    create_arrow_vertices, create_center_box_vertices, create_circle_vertices,
    create_plane_vertices, create_scale_axis_vertices,
};
use glam::{Quat, Vec3};

use gizmo_pipeline::GizmoPipeline;
use scene_pipeline::ScenePipeline;

/// Renderer構造体
pub struct Renderer {
    ctx: GpuContext,
    scene: ScenePipeline,
    gizmo: GizmoPipeline,

    // Camera
    pub camera: Camera,

    // Depth buffer
    #[allow(dead_code)]
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    // Gizmo state
    pub gizmo_state: GizmoState,
}

impl Renderer {
    /// 新しいRendererを作成（非同期）
    pub async fn create(canvas: HtmlCanvasElement) -> Result<Renderer, JsValue> {
        let ctx = GpuContext::new(canvas).await?;

        // Camera 作成
        let camera = Camera::new(ctx.aspect());
        let camera_uniform = camera.uniform();

        // Scene Pipeline 作成
        let scene = ScenePipeline::new(&ctx, bytemuck::bytes_of(&camera_uniform));

        // Gizmo Pipeline 作成
        let gizmo = GizmoPipeline::new(&ctx);

        // Depth Texture 作成
        let (depth_texture, depth_view) = depth::create_texture(&ctx.device, ctx.width(), ctx.height());

        let gizmo_state = GizmoState::default();

        console_log!("Renderer initialized successfully");

        Ok(Self {
            ctx,
            scene,
            gizmo,
            camera,
            depth_texture,
            depth_view,
            gizmo_state,
        })
    }

    /// 単一のCubeをレンダリング（固定位置）
    #[allow(dead_code)]
    pub fn render(&self) -> Result<(), JsValue> {
        let output = self
            .ctx
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to get surface texture: {:?}", e)))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Camera uniform更新
        self.ctx.queue.write_buffer(
            &self.scene.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera.uniform()),
        );

        // Default model (identity)
        let model_uniform = ModelUniform::identity();
        self.ctx.queue
            .write_buffer(&self.scene.model_buffer, 0, bytemuck::bytes_of(&model_uniform));

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

            render_pass.set_pipeline(&self.scene.pipeline);
            render_pass.set_bind_group(0, &self.scene.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.scene.model_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.scene.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.scene.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.scene.num_indices, 0, 0..1);
        }

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Canvas サイズ変更
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.ctx.resize(width, height);

            // Depth Texture再作成
            let (depth_texture, depth_view) = depth::create_texture(&self.ctx.device, width, height);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;

            // Camera aspect更新
            self.camera.set_aspect(width as f32 / height as f32);

            console_log!("Resized to {}x{}", width, height);
        }
    }

    /// 現在のサイズ取得
    pub fn width(&self) -> u32 {
        self.ctx.width()
    }

    pub fn height(&self) -> u32 {
        self.ctx.height()
    }

    /// Queue参照を取得
    #[allow(dead_code)]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.ctx.queue
    }

    /// Model Buffer参照を取得
    #[allow(dead_code)]
    pub fn model_buffer(&self) -> &wgpu::Buffer {
        &self.scene.model_buffer
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
            .ctx
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to get surface texture: {:?}", e)))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Camera uniform更新
        self.ctx.queue.write_buffer(
            &self.scene.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera.uniform()),
        );

        // 各Entityを個別のコマンドで描画
        let mut commands = Vec::new();

        for (i, model_uniform) in transforms.iter().enumerate() {
            // Model uniform更新
            self.ctx.queue
                .write_buffer(&self.scene.model_buffer, 0, bytemuck::bytes_of(model_uniform));

            let mut encoder = self
                .ctx
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

                render_pass.set_pipeline(&self.scene.pipeline);
                render_pass.set_bind_group(0, &self.scene.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.scene.model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.scene.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.scene.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.scene.num_indices, 0, 0..1);
            }

            commands.push(encoder.finish());
            // 各コマンドを個別にsubmitしてバッファ更新を反映
            self.ctx.queue.submit(commands.drain(..));
        }

        // Entityが0の場合は背景のみ描画
        if transforms.is_empty() {
            let mut encoder = self
                .ctx
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

            self.ctx.queue.submit(std::iter::once(encoder.finish()));
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
        let vertex_buffer = self.ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
        self.ctx.queue.write_buffer(
            &self.gizmo.uniform_buffer,
            0,
            bytemuck::bytes_of(&gizmo_uniform),
        );

        let mut encoder = self
            .ctx
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

            render_pass.set_pipeline(&self.gizmo.pipeline);
            render_pass.set_bind_group(0, &self.gizmo.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..vertices.len() as u32, 0..1);
        }

        self.ctx.queue.submit(std::iter::once(encoder.finish()));

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
