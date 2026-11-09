//! GPU backend for the PAL scene renderer.
//!
//! The scene graph (`FrameScene`) is backend-neutral: textured quads carry
//! tint, rotation, optional PAL raster FX, and a smooth-upscale hint. This backend maps each `SceneTexture` to an RGBA8 texture
//! (re-uploaded only when its generation changes) and emits one draw call per
//! sprite, mirroring the software compositor's source-over blending.

use std::collections::HashMap;
use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::scene::{
    letterbox_rect, DrawCommand, FrameScene, RectF, SceneTexture, SceneTextureId, SpriteDraw,
};

use super::{
    pal_device_rect_to_clip_quad, scaled_rect, shader_source, RenderOutcome, ShaderProgram,
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RasterGpuUniforms {
    params: [f32; 4],
    transform_x: [f32; 4],
    transform_y: [f32; 4],
}

impl RasterGpuUniforms {
    fn new(fx: crate::fx::RasterUniforms, quad: [[f32; 2]; 4]) -> Self {
        let row = |axis: usize| {
            [
                (quad[1][axis] - quad[0][axis]) * 0.5,
                (quad[0][axis] - quad[3][axis]) * 0.5,
                (quad[0][axis] + quad[2][axis]) * 0.5,
                0.0,
            ]
        };
        Self {
            params: [fx.offset, fx.undulate_x, fx.undulate_y, 0.0],
            transform_x: row(0),
            transform_y: row(1),
        }
    }
}

fn raster_bind_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("PAL raster parameters"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(48),
            },
            count: None,
        }],
    })
}

fn raster_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniforms: RasterGpuUniforms,
) -> wgpu::BindGroup {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("PAL raster uniforms"),
        contents: bytemuck::bytes_of(&uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("PAL raster uniforms"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

fn raster_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    texture: &wgpu::BindGroupLayout,
    uniforms: &wgpu::BindGroupLayout,
    mode: u32,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("PAL raster pipeline layout"),
        bind_group_layouts: &[Some(texture), Some(uniforms)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(ShaderProgram::Raster.label()),
        source: wgpu::ShaderSource::Wgsl(shader_source(ShaderProgram::Raster).into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("PAL raster pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(SpriteVertex::LAYOUT)],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: blend_state(mode),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn blend_state(mode: u32) -> Option<wgpu::BlendState> {
    use wgpu::{BlendComponent, BlendFactor as Factor, BlendOperation as Op};
    let (src_factor, dst_factor, operation) = match mode {
        2 => (Factor::SrcAlpha, Factor::One, Op::Add),
        3 | 5 => (Factor::Zero, Factor::Src, Op::Add),
        4 => (Factor::Zero, Factor::Dst, Op::Add),
        6 => (Factor::OneMinusDst, Factor::One, Op::Add),
        7 => (Factor::One, Factor::One, Op::Min),
        8 => (Factor::One, Factor::One, Op::Max),
        9 => (Factor::One, Factor::One, Op::Add),
        10 => (Factor::One, Factor::One, Op::ReverseSubtract),
        11 => return None,
        _ => (Factor::SrcAlpha, Factor::OneMinusSrcAlpha, Op::Add),
    };
    Some(wgpu::BlendState {
        color: BlendComponent {
            src_factor,
            dst_factor,
            operation,
        },
        alpha: wgpu::BlendComponent::OVER,
    })
}

impl SpriteVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 16,
                shader_location: 2,
            },
        ],
    };
}

const SPRITE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(sprite_texture, sprite_sampler, in.uv);
    return vec4<f32>(texel.rgb * in.color.rgb, texel.a * in.color.a);
}
"#;

struct GpuTexture {
    generation: u64,
    width: u32,
    height: u32,
    bind_nearest: wgpu::BindGroup,
    bind_linear: wgpu::BindGroup,
}

#[derive(Clone, Copy)]
enum SpanBinding {
    Nearest(SceneTextureId),
    Linear(SceneTextureId),
    White,
}

struct DrawSpan {
    raster: Option<RasterGpuUniforms>,
    binding: SpanBinding,
    first_vertex: u32,
    mode: usize,
}

pub struct WgpuBackend {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipelines: Vec<wgpu::RenderPipeline>,
    raster_pipelines: Vec<wgpu::RenderPipeline>,
    raster_bind_layout: wgpu::BindGroupLayout,
    sampler_nearest: wgpu::Sampler,
    sampler_linear: wgpu::Sampler,
    texture_bind_layout: wgpu::BindGroupLayout,
    white_bind_nearest: wgpu::BindGroup,
    textures: HashMap<SceneTextureId, GpuTexture>,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: u64,
    captured: Option<(u32, u32, Vec<u8>)>,
}

impl WgpuBackend {
    pub async fn new(window: Arc<Window>, size: PhysicalSize<u32>) -> anyhow::Result<Self> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(window.clone())));
        let surface = instance.create_surface(window)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                apply_limit_buckets: false,
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .map_err(|err| anyhow::anyhow!("no compatible GPU adapter: {err}"))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|err| anyhow::anyhow!("GPU device request failed: {err}"))?;
        let capabilities = surface.get_capabilities(&adapter);
        // Prefer non-sRGB formats so blending matches the software compositor
        // (which blends the already-sRGB-encoded scene values directly).
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| {
                matches!(
                    format,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
            .or_else(|| {
                capabilities
                    .formats
                    .iter()
                    .copied()
                    .find(|format| !format.is_srgb())
            })
            .unwrap_or(capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            color_space: wgpu::SurfaceColorSpace::Auto,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pal nearest sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pal linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let texture_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pal sprite texture layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pal sprite pipeline layout"),
            bind_group_layouts: &[Some(&texture_bind_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pal sprite shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
        });
        let pipelines = (0..12)
            .map(|mode| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("pal sprite pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[Some(SpriteVertex::LAYOUT)],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: blend_state(mode),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .collect();

        let raster_bind_layout = raster_bind_layout(&device);
        let raster_pipelines = (0..12)
            .map(|mode| {
                raster_pipeline(
                    &device,
                    format,
                    &texture_bind_layout,
                    &raster_bind_layout,
                    mode,
                )
            })
            .collect();

        let white_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pal solid quad texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &white_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let white_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let white_bind_nearest =
            Self::bind_texture(&device, &texture_bind_layout, &white_view, &sampler_nearest);

        let vertex_capacity = 4096_u64;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pal sprite vertex buffer"),
            size: vertex_capacity * std::mem::size_of::<SpriteVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        log::info!(
            "wgpu renderer initialized: adapter={:?} format={format:?} size={}x{}",
            adapter.get_info().name,
            config.width,
            config.height
        );
        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipelines,
            raster_pipelines,
            raster_bind_layout,
            sampler_nearest,
            sampler_linear,
            texture_bind_layout,
            white_bind_nearest,
            textures: HashMap::new(),
            vertex_buffer,
            vertex_capacity,
            captured: None,
        })
    }

    fn bind_texture(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pal sprite texture bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let width = size.width.max(1);
        let height = size.height.max(1);
        if width == self.config.width && height == self.config.height {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    fn sync_textures(&mut self, scene: &FrameScene) {
        self.textures.retain(|texture_id, _| {
            scene
                .textures
                .iter()
                .any(|texture| texture.id == *texture_id)
        });
        for texture in &scene.textures {
            let (width, height, _, _) = texture.device_texels();
            let needs_upload = self.textures.get(&texture.id).is_none_or(|cached| {
                cached.generation != texture.generation
                    || cached.width != width
                    || cached.height != height
            });
            if !needs_upload {
                continue;
            }
            match self.upload_texture(texture) {
                Ok(gpu_texture) => {
                    self.textures.insert(texture.id, gpu_texture);
                }
                Err(err) => {
                    log::error!("failed to upload scene texture {:?}: {err}", texture.id);
                }
            }
        }
    }

    fn upload_texture(&self, texture: &SceneTexture) -> anyhow::Result<GpuTexture> {
        let (width, height, _, pixels) = texture.device_texels();
        if width == 0 || height == 0 {
            anyhow::bail!("texture {:?} has an empty size", texture.id);
        }
        let expected = width as usize * height as usize * 4;
        if pixels.len() != expected {
            anyhow::bail!(
                "texture {:?} has {} bytes, expected {}",
                texture.id,
                pixels.len(),
                expected
            );
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let gpu_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pal scene texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(GpuTexture {
            generation: texture.generation,
            width,
            height,
            bind_nearest: Self::bind_texture(
                &self.device,
                &self.texture_bind_layout,
                &view,
                &self.sampler_nearest,
            ),
            bind_linear: Self::bind_texture(
                &self.device,
                &self.texture_bind_layout,
                &view,
                &self.sampler_linear,
            ),
        })
    }

    pub fn render(
        &mut self,
        scene: &FrameScene,
        fallback_clear: wgpu::Color,
        capture: bool,
    ) -> anyhow::Result<RenderOutcome> {
        let resolved = crate::scene::resolve_raster_stage(scene);
        let scene = resolved.as_ref();
        self.sync_textures(scene);
        // All PAL geometry is built in viewport space; the render pass
        // viewport places it letterboxed inside the surface.
        let viewport = letterbox_rect(
            [self.config.width, self.config.height],
            [scene.logical_width.max(1), scene.logical_height.max(1)],
        );
        let scale = [
            viewport.w / scene.logical_width.max(1) as f32,
            viewport.h / scene.logical_height.max(1) as f32,
        ];
        let surface_size = [viewport.w as u32, viewport.h as u32];
        let clear = scene_clear_color(scene, fallback_clear);
        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(scene.commands.len() * 6 + 6);
        let mut spans: Vec<DrawSpan> = Vec::with_capacity(scene.commands.len() + 1);
        // The scene clear color covers the PAL screen only; bars stay black.
        push_quad_vertices(
            &mut vertices,
            &mut spans,
            SpanBinding::White,
            RectF::new(0.0, 0.0, viewport.w, viewport.h),
            surface_size,
            [0.0, 0.0, 1.0, 1.0],
            [clear.r as f32, clear.g as f32, clear.b as f32, 1.0],
        );
        for command in &scene.commands {
            match command {
                DrawCommand::Sprite(sprite) => {
                    self.push_sprite(&mut vertices, &mut spans, scale, surface_size, sprite);
                }
                DrawCommand::SolidQuad(quad) => {
                    if !quad.dst.is_drawable() {
                        continue;
                    }
                    let dst_device = scaled_rect(quad.dst, scale);
                    push_quad_vertices(
                        &mut vertices,
                        &mut spans,
                        SpanBinding::White,
                        dst_device,
                        surface_size,
                        [0.0, 0.0, 1.0, 1.0],
                        quad.color,
                    );
                }
            }
        }
        if vertices.is_empty() {
            // Still present the clear color so window flashes are visible.
        }
        let required = vertices.len() as u64;
        if required > self.vertex_capacity {
            self.vertex_capacity = required.next_power_of_two();
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("pal sprite vertex buffer"),
                size: self.vertex_capacity * std::mem::size_of::<SpriteVertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if required > 0 {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }
        let raster_bindings: Vec<_> = spans
            .iter()
            .map(|span| {
                span.raster.map(|uniforms| {
                    raster_bind_group(&self.device, &self.raster_bind_layout, uniforms)
                })
            })
            .collect();
        // A diagnostic capture renders offscreen, so it does not depend on
        // the window surface being presentable (hidden windows are occluded).
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pal frame encoder"),
            });
        let pending_capture =
            capture.then(|| self.record_capture(&mut encoder, viewport, &spans, &raster_bindings));
        let (frame, outcome) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (Some(frame), RenderOutcome::Rendered),
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.reconfigure();
                (None, RenderOutcome::Reconfigured)
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                (None, RenderOutcome::Skipped)
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                anyhow::bail!("surface validation error while acquiring the next frame");
            }
        };
        if let Some(frame) = &frame {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            self.record_pass(&mut encoder, &view, viewport, &spans, &raster_bindings);
        }
        if frame.is_none() && pending_capture.is_none() {
            return Ok(outcome);
        }
        self.queue.submit([encoder.finish()]);
        if let Some(frame) = frame {
            self.queue.present(frame);
        }
        self.captured = pending_capture.and_then(|capture| match capture.read(&self.device) {
            Ok(rgba) => Some(rgba),
            Err(err) => {
                log::error!("GPU frame readback failed: {err:#}");
                None
            }
        });
        Ok(outcome)
    }

    /// The RGBA surface pixels of the last frame rendered with `capture`.
    pub fn take_captured_frame(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        self.captured.take()
    }

    fn record_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: RectF,
        spans: &[DrawSpan],
        raster_bindings: &[Option<wgpu::BindGroup>],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pal frame pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(viewport.x, viewport.y, viewport.w, viewport.h, 0.0, 1.0);
        pass.set_scissor_rect(
            viewport.x as u32,
            viewport.y as u32,
            viewport.w as u32,
            viewport.h as u32,
        );
        pass.set_pipeline(&self.pipelines[1]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        for (span_index, span) in spans.iter().enumerate() {
            let bind_group = match span.binding {
                SpanBinding::Nearest(texture_id) => self
                    .textures
                    .get(&texture_id)
                    .map(|texture| &texture.bind_nearest),
                SpanBinding::Linear(texture_id) => self
                    .textures
                    .get(&texture_id)
                    .map(|texture| &texture.bind_linear),
                SpanBinding::White => Some(&self.white_bind_nearest),
            };
            let Some(bind_group) = bind_group else {
                continue;
            };
            if let Some(raster) = &raster_bindings[span_index] {
                pass.set_pipeline(&self.raster_pipelines[span.mode]);
                pass.set_bind_group(1, raster, &[]);
            } else {
                pass.set_pipeline(&self.pipelines[span.mode]);
            }
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(span.first_vertex..span.first_vertex + 6, 0..1);
        }
    }

    /// Record the same frame into an offscreen twin of the surface and copy
    /// it to a mappable buffer, so diagnostic dumps show the real GPU output.
    fn record_capture(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        viewport: RectF,
        spans: &[DrawSpan],
        raster_bindings: &[Option<wgpu::BindGroup>],
    ) -> FrameCapture {
        let (width, height) = (self.config.width, self.config.height);
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pal frame capture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.record_pass(encoder, &view, viewport, spans, raster_bindings);
        let padded_row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pal frame capture buffer"),
            size: u64::from(padded_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            size,
        );
        FrameCapture {
            buffer,
            width,
            height,
            padded_row,
            bgra: matches!(
                self.config.format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            ),
        }
    }

    fn push_sprite(
        &self,
        vertices: &mut Vec<SpriteVertex>,
        spans: &mut Vec<DrawSpan>,
        scale: [f32; 2],
        surface_size: [u32; 2],
        sprite: &SpriteDraw,
    ) {
        if !sprite.dst.is_drawable() || !sprite.src.is_drawable() {
            return;
        }
        let Some(texture) = self.textures.get(&sprite.texture_id) else {
            log::warn!(
                "skipping sprite with missing texture {:?}; no diagnostic fallback drawn",
                sprite.texture_id
            );
            return;
        };
        let dst_device = scaled_rect(sprite.dst, scale);
        let src_w = sprite.src.w * texture.width as f32;
        let src_h = sprite.src.h * texture.height as f32;
        // Match the software compositor: smooth sampling only kicks in when
        // the sprite is actually upscaled past its native texel density.
        let smooth = sprite.raster_fx.is_none()
            && sprite.smooth_upscale
            && (dst_device.w > src_w * 1.1 || dst_device.h > src_h * 1.1);
        let mut uvs = [sprite.src.x, sprite.src.y, sprite.src.w, sprite.src.h];
        if smooth {
            // Half-texel inset keeps linear sampling inside the source cell so
            // neighboring button animation cells do not bleed in.
            let inset_u = (0.5 / texture.width as f32).min(sprite.src.w * 0.25);
            let inset_v = (0.5 / texture.height as f32).min(sprite.src.h * 0.25);
            uvs[0] += inset_u;
            uvs[1] += inset_v;
            uvs[2] -= inset_u * 2.0;
            uvs[3] -= inset_v * 2.0;
        }
        let binding = if smooth {
            SpanBinding::Linear(sprite.texture_id)
        } else {
            SpanBinding::Nearest(sprite.texture_id)
        };
        let first_vertex = vertices.len();
        push_quad_vertices(
            vertices,
            spans,
            binding,
            dst_device,
            surface_size,
            uvs,
            sprite.effective_blend_state().modulation(sprite.color),
        );
        spans.last_mut().unwrap().mode = sprite
            .effective_blend_state()
            .pipeline_index(sprite.render_mode);
        let quad = sprite.quad(false).map(|point| {
            [
                point[0] * scale[0] / surface_size[0] as f32 * 2.0 - 1.0,
                1.0 - point[1] * scale[1] / surface_size[1] as f32 * 2.0,
            ]
        });
        if let Some(fx) = sprite.raster_fx {
            spans.last_mut().unwrap().raster = Some(RasterGpuUniforms::new(fx, quad));
            let local = [[-1.0, 1.0], [1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]];
            for (vertex, index) in vertices[first_vertex..].iter_mut().zip([0, 1, 2, 0, 2, 3]) {
                vertex.position = local[index];
            }
        } else {
            for (vertex, index) in vertices[first_vertex..].iter_mut().zip([0, 1, 2, 0, 2, 3]) {
                vertex.position = quad[index];
            }
        }
    }
}

/// A frame copy waiting in a mappable buffer (rows padded for the copy).
struct FrameCapture {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_row: u32,
    bgra: bool,
}

impl FrameCapture {
    fn read(self, device: &wgpu::Device) -> anyhow::Result<(u32, u32, Vec<u8>)> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|err| anyhow::anyhow!("{err}"))?;
        receiver.recv()??;
        let mapped = self
            .buffer
            .get_mapped_range(..)
            .map_err(|err| anyhow::anyhow!("{err:?}"))?;
        let mut rgba = Vec::with_capacity(self.width as usize * self.height as usize * 4);
        for row in mapped.chunks_exact(self.padded_row as usize) {
            for pixel in row[..self.width as usize * 4].chunks_exact(4) {
                let (r, g, b) = if self.bgra {
                    (pixel[2], pixel[1], pixel[0])
                } else {
                    (pixel[0], pixel[1], pixel[2])
                };
                rgba.extend_from_slice(&[r, g, b, 255]);
            }
        }
        drop(mapped);
        self.buffer.unmap();
        Ok((self.width, self.height, rgba))
    }
}

fn push_quad_vertices(
    vertices: &mut Vec<SpriteVertex>,
    spans: &mut Vec<DrawSpan>,
    binding: SpanBinding,
    dst_device: crate::scene::RectF,
    surface_size: [u32; 2],
    uvs: [f32; 4],
    color: [f32; 4],
) {
    let quad = pal_device_rect_to_clip_quad(dst_device, surface_size);
    let color = [
        color[0].clamp(0.0, 1.0),
        color[1].clamp(0.0, 1.0),
        color[2].clamp(0.0, 1.0),
        color[3].clamp(0.0, 1.0),
    ];
    let (u0, v0) = (uvs[0], uvs[1]);
    let (u1, v1) = (uvs[0] + uvs[2], uvs[1] + uvs[3]);
    let corners = [
        (quad[0], [u0, v0]),
        (quad[1], [u1, v0]),
        (quad[2], [u1, v1]),
        (quad[3], [u0, v1]),
    ];
    spans.push(DrawSpan {
        raster: None,
        binding,
        first_vertex: vertices.len() as u32,
        mode: 1,
    });
    for index in [0_usize, 1, 2, 0, 2, 3] {
        let (position, uv) = corners[index];
        vertices.push(SpriteVertex {
            position,
            uv,
            color,
        });
    }
}

fn scene_clear_color(scene: &FrameScene, fallback: wgpu::Color) -> wgpu::Color {
    let [r, g, b, a] = scene.clear_color;
    if [r, g, b, a].iter().all(|v| v.is_finite()) {
        wgpu::Color { r, g, b, a }
    } else {
        fallback
    }
}

#[cfg(test)]
#[path = "raster_tests.rs"]
mod raster_tests;
