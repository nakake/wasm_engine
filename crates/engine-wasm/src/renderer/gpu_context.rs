//! GPUコンテキストモジュール
//!
//! WebGPUのDevice, Queue, Surfaceを管理

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::utils::console_log;

/// GPUコンテキスト
/// WebGPUの基本リソースを保持
pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: (u32, u32),
}

impl GpuContext {
    /// 新しいGPUコンテキストを作成（非同期）
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
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

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size: (width, height),
        })
    }

    /// リサイズ
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = (width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// 幅を取得
    pub fn width(&self) -> u32 {
        self.size.0
    }

    /// 高さを取得
    pub fn height(&self) -> u32 {
        self.size.1
    }

    /// アスペクト比を取得
    pub fn aspect(&self) -> f32 {
        self.size.0 as f32 / self.size.1 as f32
    }
}
