#[cfg(target_arch = "wasm32")]
use js_sys::JSON;

#[cfg(target_arch = "wasm32")]
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct RendererInfo {
    adapter_name: String,
    backend: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScenePayload {
    vertices: Vec<Vertex>,
    #[serde(default)]
    edge_vertices: Vec<Vertex>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraPayload {
    view_projection: [f32; 16],
}

#[cfg(target_arch = "wasm32")]
#[repr(C)]
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[cfg(target_arch = "wasm32")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CameraUniform {
    view_projection: [f32; 16],
}

#[cfg(target_arch = "wasm32")]
struct DepthTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct RendererHandle {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    fill_pipeline: wgpu::RenderPipeline,
    edge_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    fill_vertex_buffer: wgpu::Buffer,
    fill_vertex_count: u32,
    edge_vertex_buffer: wgpu::Buffer,
    edge_vertex_count: u32,
    depth_target: DepthTarget,
}

#[cfg(target_arch = "wasm32")]
const SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

#[cfg(target_arch = "wasm32")]
fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

#[cfg(target_arch = "wasm32")]
fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[cfg(target_arch = "wasm32")]
fn parse_js_value<T: for<'de> Deserialize<'de>>(value: JsValue) -> Result<T, JsValue> {
    let json = JSON::stringify(&value)
        .map_err(|error| js_error(format!("Failed to stringify JsValue: {error:?}")))?
        .as_string()
        .ok_or_else(|| js_error("Failed to read JSON string from JsValue"))?;

    serde_json::from_str::<T>(&json).map_err(|error| js_error(format!("Failed to parse JSON payload: {error}")))
}

#[cfg(target_arch = "wasm32")]
fn bytes_of_slice<T>(values: &[T]) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            values.as_ptr() as *const u8,
            core::mem::size_of_val(values),
        )
    }
}

#[cfg(target_arch = "wasm32")]
fn bytes_of_value<T>(value: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts((value as *const T).cast::<u8>(), core::mem::size_of::<T>()) }
}

#[cfg(target_arch = "wasm32")]
fn create_depth_target(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gloq-space-depth"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    DepthTarget {
        _texture: texture,
        view,
    }
}

#[cfg(target_arch = "wasm32")]
fn empty_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gloq-space-empty-vertices"),
        size: core::mem::size_of::<Vertex>() as u64,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    })
}

#[cfg(target_arch = "wasm32")]
fn create_render_pipeline(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    shader: &wgpu::ShaderModule,
    pipeline_layout: &wgpu::PipelineLayout,
    label: &'static str,
    topology: wgpu::PrimitiveTopology,
    depth_write_enabled: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: core::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: core::mem::size_of::<[f32; 3]>() as u64,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl RendererInfo {
    #[wasm_bindgen(getter)]
    pub fn adapter_name(&self) -> String {
        self.adapter_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn backend(&self) -> String {
        self.backend.clone()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn probe_webgpu() -> Result<RendererInfo, JsValue> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let info = adapter.get_info();

    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))?;

    Ok(RendererInfo {
        adapter_name: info.name,
        backend: format!("{:?}", info.backend),
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn create_renderer(canvas: HtmlCanvasElement) -> Result<RendererHandle, JsValue> {
    let instance = wgpu::Instance::default();
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|error| js_error(format!("Failed to create WebGPU surface: {error}")))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .map_err(|error| js_error(format!("Failed to request WebGPU adapter: {error}")))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("gloq-space-renderer"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|error| js_error(format!("Failed to request WebGPU device: {error}")))?;
    let mut config = surface
        .get_default_config(&adapter, width, height)
        .ok_or_else(|| js_error("Surface configuration is not supported by the selected adapter"))?;
    config.width = width;
    config.height = height;
    surface.configure(&device, &config);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gloq-space-shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });
    let camera_uniform = CameraUniform {
        view_projection: identity_matrix(),
    };
    let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gloq-space-camera"),
        size: core::mem::size_of::<CameraUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&camera_buffer, 0, bytes_of_value(&camera_uniform));

    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gloq-space-camera-layout"),
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
    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gloq-space-camera-bind-group"),
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gloq-space-pipeline-layout"),
        bind_group_layouts: &[&camera_bind_group_layout],
        push_constant_ranges: &[],
    });
    let fill_pipeline = create_render_pipeline(
        &device,
        &config,
        &shader,
        &pipeline_layout,
        "gloq-space-fill-pipeline",
        wgpu::PrimitiveTopology::TriangleList,
        true,
    );
    let edge_pipeline = create_render_pipeline(
        &device,
        &config,
        &shader,
        &pipeline_layout,
        "gloq-space-edge-pipeline",
        wgpu::PrimitiveTopology::LineList,
        false,
    );
    let depth_target = create_depth_target(&device, &config);
    let fill_vertex_buffer = empty_vertex_buffer(&device);
    let edge_vertex_buffer = empty_vertex_buffer(&device);

    Ok(RendererHandle {
        surface,
        device,
        queue,
        config,
        fill_pipeline,
        edge_pipeline,
        camera_buffer,
        camera_bind_group,
        fill_vertex_buffer,
        fill_vertex_count: 0,
        edge_vertex_buffer,
        edge_vertex_count: 0,
        depth_target,
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl RendererHandle {
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        let next_width = width.max(1);
        let next_height = height.max(1);

        if self.config.width == next_width && self.config.height == next_height {
            return Ok(());
        }

        self.config.width = next_width;
        self.config.height = next_height;
        self.surface.configure(&self.device, &self.config);
        self.depth_target = create_depth_target(&self.device, &self.config);
        Ok(())
    }

    pub fn set_scene(&mut self, scene: JsValue) -> Result<(), JsValue> {
        let payload: ScenePayload = parse_js_value(scene)?;
        let vertices = payload.vertices;
        let edge_vertices = payload.edge_vertices;

        self.fill_vertex_count = vertices.len() as u32;
        self.fill_vertex_buffer = if vertices.is_empty() {
            empty_vertex_buffer(&self.device)
        } else {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gloq-space-vertices"),
                size: bytes_of_slice(&vertices).len() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        if !vertices.is_empty() {
            self.queue
                .write_buffer(&self.fill_vertex_buffer, 0, bytes_of_slice(&vertices));
        }

        self.edge_vertex_count = edge_vertices.len() as u32;
        self.edge_vertex_buffer = if edge_vertices.is_empty() {
            empty_vertex_buffer(&self.device)
        } else {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gloq-space-edge-vertices"),
                size: bytes_of_slice(&edge_vertices).len() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        if !edge_vertices.is_empty() {
            self.queue
                .write_buffer(&self.edge_vertex_buffer, 0, bytes_of_slice(&edge_vertices));
        }

        Ok(())
    }

    pub fn set_camera(&mut self, camera: JsValue) -> Result<(), JsValue> {
        let payload: CameraPayload = parse_js_value(camera)?;
        let uniform = CameraUniform {
            view_projection: payload.view_projection,
        };

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytes_of_value(&uniform));
        Ok(())
    }

    pub fn render(&mut self) -> Result<(), JsValue> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .map_err(|error| js_error(format!("Failed to reacquire swapchain texture: {error}")))?
            }
            Err(error) => {
                return Err(js_error(format!(
                    "Failed to acquire next swapchain texture: {error}"
                )))
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gloq-space-render"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gloq-space-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.06,
                            g: 0.10,
                            b: 0.14,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_target.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            if self.fill_vertex_count > 0 {
                pass.set_pipeline(&self.fill_pipeline);
                pass.set_vertex_buffer(0, self.fill_vertex_buffer.slice(..));
                pass.draw(0..self.fill_vertex_count, 0..1);
            }

            if self.edge_vertex_count > 0 {
                pass.set_pipeline(&self.edge_pipeline);
                pass.set_vertex_buffer(0, self.edge_vertex_buffer.slice(..));
                pass.draw(0..self.edge_vertex_count, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn native_build_note() -> &'static str {
    "render-wasm is intended for the wasm32-unknown-unknown target"
}
