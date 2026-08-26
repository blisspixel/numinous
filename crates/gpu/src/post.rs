//! Feature-gated HDR bloom spike for the Sensory Lift.

use super::{
    GpuAdapterInfo, GpuContext, RenderError, capture_device_errors, copy_mapped_bytes,
    frame_size_supported, validate_map_completion,
};
use bytemuck::{Pod, Zeroable};
use std::time::Instant;
use wgpu::util::DeviceExt;

const BYTES_PER_PIXEL: u32 = 4;
const COPY_ROW_ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
const TIMESTAMP_COUNT: u32 = 2;
const TIMESTAMP_BYTES: u64 = TIMESTAMP_COUNT as u64 * std::mem::size_of::<u64>() as u64;

/// Adapter support relevant to the post-processing spike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostCapabilities {
    /// Whether `Rgba16Float` can be sampled and used as a render target.
    pub hdr_render_target: bool,
    /// Whether `Rgba16Float` supports linear filtering.
    pub hdr_filterable: bool,
    /// Whether this adapter exposes GPU timestamp queries.
    pub timestamp_queries: bool,
}

impl PostCapabilities {
    fn from_context(context: &GpuContext) -> Self {
        let usages = context.rgba16_float_features.allowed_usages;
        Self {
            hdr_render_target: usages.contains(
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            hdr_filterable: context
                .rgba16_float_features
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE),
            timestamp_queries: context.timestamp_queries,
        }
    }

    fn validate(self) -> Result<(), RenderError> {
        if !self.hdr_render_target {
            return Err(RenderError::DeviceLimit(
                "sampled Rgba16Float render-target support",
            ));
        }
        if !self.hdr_filterable {
            return Err(RenderError::DeviceLimit(
                "filterable Rgba16Float texture support",
            ));
        }
        Ok(())
    }
}

/// One completed post-processed frame and its optional device timing.
#[derive(Debug)]
pub struct PostFrame {
    /// Tightly packed output in sRGB RGBA byte order.
    pub rgba: Vec<u8>,
    /// Device time from the first render pass to the last, when supported.
    pub gpu_time_ms: Option<f64>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PostParams {
    inverse_source_size: [f32; 2],
    threshold: f32,
    bloom_strength: f32,
    exposure: f32,
    padding: [f32; 3],
}

impl PostParams {
    fn new(width: u32, height: u32) -> Self {
        Self {
            inverse_source_size: [1.0 / width as f32, 1.0 / height as f32],
            threshold: 0.72,
            bloom_strength: 0.78,
            exposure: 1.08,
            padding: [0.0; 3],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PostLayout {
    width: u32,
    height: u32,
    half_width: u32,
    half_height: u32,
    tight_row_bytes: u32,
    padded_row_bytes: u32,
    tight_byte_len: usize,
    padded_byte_len: u64,
}

impl PostLayout {
    fn validate(width: u32, height: u32, limits: &wgpu::Limits) -> Result<Self, RenderError> {
        if !frame_size_supported(width as usize, height as usize)
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
        {
            return Err(RenderError::InvalidDimensions { width, height });
        }
        let tight_row_bytes = width
            .checked_mul(BYTES_PER_PIXEL)
            .ok_or(RenderError::InvalidDimensions { width, height })?;
        let padded_row_bytes = tight_row_bytes.div_ceil(COPY_ROW_ALIGNMENT) * COPY_ROW_ALIGNMENT;
        let padded_byte_len = u64::from(padded_row_bytes)
            .checked_mul(u64::from(height))
            .ok_or(RenderError::InvalidDimensions { width, height })?;
        if padded_byte_len > limits.max_buffer_size {
            return Err(RenderError::DeviceLimit("maximum readback buffer size"));
        }
        let tight_byte_len = usize::try_from(
            u64::from(tight_row_bytes)
                .checked_mul(u64::from(height))
                .ok_or(RenderError::InvalidDimensions { width, height })?,
        )
        .map_err(|_| RenderError::InvalidDimensions { width, height })?;
        Ok(Self {
            width,
            height,
            half_width: width.div_ceil(2),
            half_height: height.div_ceil(2),
            tight_row_bytes,
            padded_row_bytes,
            tight_byte_len,
            padded_byte_len,
        })
    }
}

struct Pipelines {
    linearize: wgpu::RenderPipeline,
    bright_pass: wgpu::RenderPipeline,
    blur_horizontal: wgpu::RenderPipeline,
    blur_vertical: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
}

impl Pipelines {
    fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sensory-post"),
            source: wgpu::ShaderSource::Wgsl(include_str!("post.wgsl").into()),
        });
        Self {
            linearize: create_pipeline(
                device,
                &module,
                "sensory-linearize",
                "linearize",
                wgpu::TextureFormat::Rgba16Float,
            ),
            bright_pass: create_pipeline(
                device,
                &module,
                "sensory-bright-pass",
                "bright_pass",
                wgpu::TextureFormat::Rgba16Float,
            ),
            blur_horizontal: create_pipeline(
                device,
                &module,
                "sensory-blur-horizontal",
                "blur_horizontal",
                wgpu::TextureFormat::Rgba16Float,
            ),
            blur_vertical: create_pipeline(
                device,
                &module,
                "sensory-blur-vertical",
                "blur_vertical",
                wgpu::TextureFormat::Rgba16Float,
            ),
            composite: create_pipeline(
                device,
                &module,
                "sensory-composite",
                "composite",
                output_format,
            ),
        }
    }
}

struct PostResources {
    layout: PostLayout,
    input_texture: wgpu::Texture,
    _hdr_texture: wgpu::Texture,
    _bright_texture: wgpu::Texture,
    _blur_texture: wgpu::Texture,
    _bloom_texture: wgpu::Texture,
    linearize_bind: wgpu::BindGroup,
    bright_bind: wgpu::BindGroup,
    horizontal_bind: wgpu::BindGroup,
    vertical_bind: wgpu::BindGroup,
    composite_bind: wgpu::BindGroup,
    hdr_view: wgpu::TextureView,
    bright_view: wgpu::TextureView,
    blur_view: wgpu::TextureView,
    bloom_view: wgpu::TextureView,
    timestamps: Option<TimestampResources>,
}

struct OffscreenResources {
    post: PostResources,
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,
    readback: wgpu::Buffer,
}

struct TimestampResources {
    query_set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
}

impl PostResources {
    fn new(
        device: &wgpu::Device,
        pipelines: &Pipelines,
        sampler: &wgpu::Sampler,
        layout: PostLayout,
        timestamp_queries: bool,
    ) -> Self {
        let input_texture = create_texture(
            device,
            "sensory-input-srgb",
            layout.width,
            layout.height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let hdr_texture = create_post_texture(
            device,
            "sensory-hdr",
            layout.width,
            layout.height,
            wgpu::TextureUsages::empty(),
        );
        let bright_texture = create_post_texture(
            device,
            "sensory-bright",
            layout.half_width,
            layout.half_height,
            wgpu::TextureUsages::empty(),
        );
        let blur_texture = create_post_texture(
            device,
            "sensory-blur-horizontal",
            layout.half_width,
            layout.half_height,
            wgpu::TextureUsages::empty(),
        );
        let bloom_texture = create_post_texture(
            device,
            "sensory-bloom",
            layout.half_width,
            layout.half_height,
            wgpu::TextureUsages::empty(),
        );
        let input_view = input_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bright_view = bright_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let blur_view = blur_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bloom_view = bloom_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let full_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sensory-full-params"),
            contents: bytemuck::bytes_of(&PostParams::new(layout.width, layout.height)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let half_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sensory-half-params"),
            contents: bytemuck::bytes_of(&PostParams::new(layout.half_width, layout.half_height)),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let linearize_bind = create_source_bind(
            device,
            &pipelines.linearize,
            "sensory-linearize-bind",
            &input_view,
            sampler,
            &full_params,
        );
        let bright_bind = create_source_bind(
            device,
            &pipelines.bright_pass,
            "sensory-bright-bind",
            &hdr_view,
            sampler,
            &full_params,
        );
        let horizontal_bind = create_source_bind(
            device,
            &pipelines.blur_horizontal,
            "sensory-horizontal-bind",
            &bright_view,
            sampler,
            &half_params,
        );
        let vertical_bind = create_source_bind(
            device,
            &pipelines.blur_vertical,
            "sensory-vertical-bind",
            &blur_view,
            sampler,
            &half_params,
        );
        let composite_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sensory-composite-bind"),
            layout: &pipelines.composite.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: full_params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&bloom_view),
                },
            ],
        });
        let timestamps = timestamp_queries.then(|| TimestampResources::new(device));

        Self {
            layout,
            input_texture,
            _hdr_texture: hdr_texture,
            _bright_texture: bright_texture,
            _blur_texture: blur_texture,
            _bloom_texture: bloom_texture,
            linearize_bind,
            bright_bind,
            horizontal_bind,
            vertical_bind,
            composite_bind,
            hdr_view,
            bright_view,
            blur_view,
            bloom_view,
            timestamps,
        }
    }
}

impl OffscreenResources {
    fn new(
        device: &wgpu::Device,
        pipelines: &Pipelines,
        sampler: &wgpu::Sampler,
        layout: PostLayout,
        timestamp_queries: bool,
    ) -> Self {
        let post = PostResources::new(device, pipelines, sampler, layout, timestamp_queries);
        let output_texture = create_texture(
            device,
            "sensory-output-srgb",
            layout.width,
            layout.height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sensory-output-readback"),
            size: layout.padded_byte_len,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self {
            post,
            output_texture,
            output_view,
            readback,
        }
    }
}

impl TimestampResources {
    fn new(device: &wgpu::Device) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("sensory-timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: TIMESTAMP_COUNT,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sensory-timestamp-resolve"),
            size: TIMESTAMP_BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sensory-timestamp-readback"),
            size: TIMESTAMP_BYTES,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self {
            query_set,
            resolve,
            readback,
        }
    }
}

/// A reusable sRGB to linear HDR, half-resolution bloom, and tone-map pipeline.
///
/// The spike deliberately reads its final frame back for validation and timing.
/// A production surface integration can consume the output texture directly.
pub struct SensoryPostRenderer {
    context: GpuContext,
    capabilities: PostCapabilities,
    pipelines: Pipelines,
    sampler: wgpu::Sampler,
    resources: Option<OffscreenResources>,
}

impl SensoryPostRenderer {
    /// Build a post renderer on the best available adapter.
    ///
    /// # Errors
    /// Returns an error when no adapter is available or when its HDR texture
    /// capabilities cannot support this pipeline.
    pub fn new() -> Result<Self, String> {
        let context = GpuContext::new()?;
        let capabilities = PostCapabilities::from_context(&context);
        capabilities.validate().map_err(|error| error.to_string())?;
        let pipelines = Pipelines::new(&context.device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let sampler = create_linear_sampler(&context.device);
        Ok(Self {
            context,
            capabilities,
            pipelines,
            sampler,
            resources: None,
        })
    }

    /// The human-readable adapter name.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        self.context.adapter_name()
    }

    /// The graphics backend in use.
    #[must_use]
    pub fn backend(&self) -> &str {
        self.context.backend()
    }

    /// Diagnostic identity of the selected adapter and driver.
    #[must_use]
    pub const fn adapter_info(&self) -> &GpuAdapterInfo {
        self.context.adapter_info()
    }

    /// Relevant adapter capabilities checked before pipeline creation.
    #[must_use]
    pub const fn capabilities(&self) -> PostCapabilities {
        self.capabilities
    }

    /// Apply HDR bright-pass bloom and tone mapping to one sRGB RGBA frame.
    ///
    /// Textures, bind groups, uniforms, query buffers, and readback buffers are
    /// reused until the dimensions change.
    ///
    /// # Errors
    /// Returns a typed error for invalid input, unsupported limits, mapping
    /// failures, or device validation failures.
    pub fn process_rgba(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<PostFrame, RenderError> {
        let device = self.context.device.clone();
        let layout = PostLayout::validate(width, height, &device.limits())?;
        if rgba.len() != layout.tight_byte_len {
            return Err(RenderError::InvalidInputLength {
                expected: layout.tight_byte_len,
                actual: rgba.len(),
            });
        }
        let recreate = !matches!(
            &self.resources,
            Some(resources)
                if resources.post.layout.width == width && resources.post.layout.height == height
        );
        if recreate {
            self.resources = Some(OffscreenResources::new(
                &device,
                &self.pipelines,
                &self.sampler,
                layout,
                self.capabilities.timestamp_queries,
            ));
        }
        capture_device_errors(&device, || self.process_frame(&device, rgba))
    }

    fn process_frame(
        &mut self,
        device: &wgpu::Device,
        rgba: &[u8],
    ) -> Result<PostFrame, RenderError> {
        let resources = self.resources.as_ref().ok_or(RenderError::HostAllocation)?;
        let layout = resources.post.layout;
        write_input(&self.context.queue, &resources.post, rgba);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sensory-post-frame"),
        });
        encode_post_passes(
            &mut encoder,
            &self.pipelines,
            &resources.post,
            &resources.output_view,
            true,
        );
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &resources.output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &resources.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(layout.padded_row_bytes),
                    rows_per_image: Some(layout.height),
                },
            },
            wgpu::Extent3d {
                width: layout.width,
                height: layout.height,
                depth_or_array_layers: 1,
            },
        );
        if let Some(timestamps) = &resources.post.timestamps {
            encoder.resolve_query_set(
                &timestamps.query_set,
                0..TIMESTAMP_COUNT,
                &timestamps.resolve,
                0,
            );
            encoder.copy_buffer_to_buffer(
                &timestamps.resolve,
                0,
                &timestamps.readback,
                0,
                TIMESTAMP_BYTES,
            );
        }
        self.context.queue.submit(Some(encoder.finish()));

        let padded = read_mapped_buffer(device, &resources.readback)?;
        let rgba = unpack_rows(&padded, layout)?;
        let gpu_time_ms = match &resources.post.timestamps {
            Some(timestamps) => {
                let bytes = read_mapped_buffer(device, &timestamps.readback)?;
                Some(timestamp_duration_ms(
                    &bytes,
                    self.context.queue.get_timestamp_period(),
                )?)
            }
            None => None,
        };
        Ok(PostFrame { rgba, gpu_time_ms })
    }
}

/// Timing for one frame rendered directly into a presentation surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorySurfaceFrame {
    /// Time spent acquiring the next swapchain texture.
    pub acquire_ms: f64,
    /// Time from successful acquisition through the queue presentation request.
    pub render_and_present_ms: f64,
    /// Total host time from acquire request through the presentation request.
    pub boundary_ms: f64,
    /// Whether the acquired texture was usable but no longer optimal.
    pub suboptimal: bool,
}

/// A reusable Sensory Lift pipeline that tone maps directly into a swapchain.
///
/// This renderer keeps the shipped app path unchanged while measuring the real
/// window surface boundary. Its timing ends when [`wgpu::Queue::present`] returns;
/// it does not claim compositor or display scanout latency.
pub struct SensorySurfaceRenderer<'window> {
    surface: wgpu::Surface<'window>,
    config: wgpu::SurfaceConfiguration,
    context: GpuContext,
    capabilities: PostCapabilities,
    pipelines: Pipelines,
    sampler: wgpu::Sampler,
    resources: Option<PostResources>,
}

impl<'window> SensorySurfaceRenderer<'window> {
    /// Create and configure a renderer for a window or another surface target.
    ///
    /// # Errors
    /// Returns an error when surface creation, adapter selection, the requested
    /// dimensions, or the HDR and sRGB format requirements cannot be satisfied.
    pub fn new(
        target: impl Into<wgpu::SurfaceTarget<'window>>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(target)
            .map_err(|error| format!("failed to create GPU surface: {error}"))?;
        let adapter = GpuContext::request_adapter(&instance, Some(&surface))?;
        let surface_capabilities = surface.get_capabilities(&adapter);
        let format = select_surface_format(&surface_capabilities.formats)
            .ok_or("GPU surface offers no sRGB render format")?;
        let present_mode = select_present_mode(&surface_capabilities.present_modes)
            .ok_or("GPU surface offers no presentation mode")?;
        let alpha_mode = surface_capabilities
            .alpha_modes
            .first()
            .copied()
            .ok_or("GPU surface offers no alpha mode")?;
        let context = GpuContext::from_adapter(adapter)?;
        PostLayout::validate(width, height, &context.device.limits())
            .map_err(|error| error.to_string())?;
        let capabilities = PostCapabilities::from_context(&context);
        capabilities.validate().map_err(|error| error.to_string())?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            desired_maximum_frame_latency: 1,
            present_mode,
            alpha_mode,
            view_formats: Vec::new(),
        };
        surface.configure(&context.device, &config);
        let pipelines = Pipelines::new(&context.device, format);
        let sampler = create_linear_sampler(&context.device);
        Ok(Self {
            surface,
            config,
            context,
            capabilities,
            pipelines,
            sampler,
            resources: None,
        })
    }

    /// The human-readable adapter name.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        self.context.adapter_name()
    }

    /// The graphics backend in use.
    #[must_use]
    pub fn backend(&self) -> &str {
        self.context.backend()
    }

    /// Diagnostic identity of the selected adapter and driver.
    #[must_use]
    pub const fn adapter_info(&self) -> &GpuAdapterInfo {
        self.context.adapter_info()
    }

    /// Relevant adapter capabilities checked before pipeline creation.
    #[must_use]
    pub const fn capabilities(&self) -> PostCapabilities {
        self.capabilities
    }

    /// The negotiated sRGB swapchain format.
    #[must_use]
    pub const fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// The negotiated presentation mode.
    #[must_use]
    pub const fn present_mode(&self) -> wgpu::PresentMode {
        self.config.present_mode
    }

    /// The configured surface dimensions.
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Reconfigure the surface and discard size-specific post resources.
    ///
    /// # Errors
    /// Returns an error when the dimensions exceed the product or device limit.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        PostLayout::validate(width, height, &self.context.device.limits())?;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.context.device, &self.config);
        self.resources = None;
        Ok(())
    }

    /// Render a host sRGB RGBA frame directly into the next swapchain texture.
    ///
    /// # Errors
    /// Returns a typed error for invalid input, unsupported dimensions, or a
    /// surface that is temporarily or permanently unavailable.
    pub fn present_rgba(&mut self, rgba: &[u8]) -> Result<SensorySurfaceFrame, RenderError> {
        let layout = PostLayout::validate(
            self.config.width,
            self.config.height,
            &self.context.device.limits(),
        )?;
        if rgba.len() != layout.tight_byte_len {
            return Err(RenderError::InvalidInputLength {
                expected: layout.tight_byte_len,
                actual: rgba.len(),
            });
        }
        let recreate = !matches!(
            &self.resources,
            Some(resources)
                if resources.layout.width == layout.width
                    && resources.layout.height == layout.height
        );
        if recreate {
            self.resources = Some(PostResources::new(
                &self.context.device,
                &self.pipelines,
                &self.sampler,
                layout,
                false,
            ));
        }
        self.present_frame(rgba)
    }

    fn present_frame(&mut self, rgba: &[u8]) -> Result<SensorySurfaceFrame, RenderError> {
        let started = Instant::now();
        let (surface_texture, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => (texture, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => (texture, true),
            wgpu::CurrentSurfaceTexture::Timeout => return Err(RenderError::SurfaceTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Err(RenderError::SurfaceOccluded),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.context.device, &self.config);
                return Err(RenderError::SurfaceOutdated);
            }
            wgpu::CurrentSurfaceTexture::Lost => return Err(RenderError::SurfaceLost),
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::SurfaceValidation);
            }
        };
        let acquired = Instant::now();
        let resources = self.resources.as_ref().ok_or(RenderError::HostAllocation)?;
        write_input(&self.context.queue, resources, rgba);
        let output_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sensory-surface-frame"),
                });
        encode_post_passes(
            &mut encoder,
            &self.pipelines,
            resources,
            &output_view,
            false,
        );
        self.context.queue.submit(Some(encoder.finish()));
        self.context.queue.present(surface_texture);
        let presented = Instant::now();
        if suboptimal {
            self.surface.configure(&self.context.device, &self.config);
        }
        Ok(SensorySurfaceFrame {
            acquire_ms: duration_ms(acquired.duration_since(started)),
            render_and_present_ms: duration_ms(presented.duration_since(acquired)),
            boundary_ms: duration_ms(presented.duration_since(started)),
            suboptimal,
        })
    }
}

fn select_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    formats.iter().copied().find(wgpu::TextureFormat::is_srgb)
}

fn select_present_mode(present_modes: &[wgpu::PresentMode]) -> Option<wgpu::PresentMode> {
    present_modes
        .contains(&wgpu::PresentMode::Fifo)
        .then_some(wgpu::PresentMode::Fifo)
        .or_else(|| present_modes.first().copied())
}

fn duration_ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn create_linear_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("sensory-linear-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..wgpu::SamplerDescriptor::default()
    })
}

fn write_input(queue: &wgpu::Queue, resources: &PostResources, rgba: &[u8]) {
    let layout = resources.layout;
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &resources.input_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(layout.tight_row_bytes),
            rows_per_image: Some(layout.height),
        },
        wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
    );
}

fn encode_post_passes(
    encoder: &mut wgpu::CommandEncoder,
    pipelines: &Pipelines,
    resources: &PostResources,
    output_view: &wgpu::TextureView,
    write_timestamps: bool,
) {
    let first_timestamp = write_timestamps
        .then_some(resources.timestamps.as_ref())
        .flatten()
        .map(|timestamps| wgpu::RenderPassTimestampWrites {
            query_set: &timestamps.query_set,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: None,
        });
    draw_pass(
        encoder,
        "sensory-linearize-pass",
        &pipelines.linearize,
        &resources.linearize_bind,
        &resources.hdr_view,
        first_timestamp,
    );
    draw_pass(
        encoder,
        "sensory-bright-pass",
        &pipelines.bright_pass,
        &resources.bright_bind,
        &resources.bright_view,
        None,
    );
    draw_pass(
        encoder,
        "sensory-horizontal-pass",
        &pipelines.blur_horizontal,
        &resources.horizontal_bind,
        &resources.blur_view,
        None,
    );
    draw_pass(
        encoder,
        "sensory-vertical-pass",
        &pipelines.blur_vertical,
        &resources.vertical_bind,
        &resources.bloom_view,
        None,
    );
    let final_timestamp = write_timestamps
        .then_some(resources.timestamps.as_ref())
        .flatten()
        .map(|timestamps| wgpu::RenderPassTimestampWrites {
            query_set: &timestamps.query_set,
            beginning_of_pass_write_index: None,
            end_of_pass_write_index: Some(1),
        });
    draw_pass(
        encoder,
        "sensory-composite-pass",
        &pipelines.composite,
        &resources.composite_bind,
        output_view,
        final_timestamp,
    );
}

fn create_pipeline(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    label: &'static str,
    entry_point: &'static str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: None,
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(entry_point),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_texture(
    device: &wgpu::Device,
    label: &'static str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn create_post_texture(
    device: &wgpu::Device,
    label: &'static str,
    width: u32,
    height: u32,
    extra_usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    create_texture(
        device,
        label,
        width,
        height,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | extra_usage,
    )
}

fn create_source_bind(
    device: &wgpu::Device,
    pipeline: &wgpu::RenderPipeline,
    label: &'static str,
    source: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

fn draw_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &'static str,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    target: &wgpu::TextureView,
    timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
) {
    let attachment = wgpu::RenderPassColorAttachment {
        view: target,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    };
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(attachment)],
        depth_stencil_attachment: None,
        timestamp_writes,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}

fn read_mapped_buffer(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, RenderError> {
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let map_result = match device.poll(wgpu::PollType::wait_indefinitely()) {
        Ok(poll) => validate_map_completion(Ok(poll), rx.recv()),
        Err(error) => Err(RenderError::Poll(error)),
    };
    if let Err(error) = map_result {
        buffer.unmap();
        return Err(error);
    }
    let bytes = {
        let mapped = slice.get_mapped_range().map_err(RenderError::MapRange)?;
        copy_mapped_bytes(&mapped, mapped.len())
    };
    buffer.unmap();
    bytes
}

fn unpack_rows(padded: &[u8], layout: PostLayout) -> Result<Vec<u8>, RenderError> {
    if padded.len() != layout.padded_byte_len as usize {
        return Err(RenderError::InvalidInputLength {
            expected: layout.padded_byte_len as usize,
            actual: padded.len(),
        });
    }
    let mut tight = Vec::new();
    tight
        .try_reserve_exact(layout.tight_byte_len)
        .map_err(|_| RenderError::HostAllocation)?;
    let padded_row = layout.padded_row_bytes as usize;
    let tight_row = layout.tight_row_bytes as usize;
    for row in padded.chunks_exact(padded_row) {
        tight.extend_from_slice(&row[..tight_row]);
    }
    Ok(tight)
}

fn timestamp_duration_ms(bytes: &[u8], period_ns: f32) -> Result<f64, RenderError> {
    if bytes.len() != TIMESTAMP_BYTES as usize {
        return Err(RenderError::InvalidInputLength {
            expected: TIMESTAMP_BYTES as usize,
            actual: bytes.len(),
        });
    }
    let start = u64::from_le_bytes(bytes[0..8].try_into().expect("eight timestamp bytes"));
    let end = u64::from_le_bytes(bytes[8..16].try_into().expect("eight timestamp bytes"));
    Ok(end.saturating_sub(start) as f64 * f64::from(period_ns) / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::{
        PostLayout, select_present_mode, select_surface_format, timestamp_duration_ms, unpack_rows,
    };

    #[test]
    fn row_layout_is_aligned_and_unpacks_without_padding() {
        let layout = PostLayout::validate(65, 2, &wgpu::Limits::default()).expect("layout");
        assert_eq!(layout.tight_row_bytes, 260);
        assert_eq!(layout.padded_row_bytes, 512);
        let mut padded = vec![0; layout.padded_byte_len as usize];
        padded[..260].fill(1);
        padded[512..772].fill(2);
        let tight = unpack_rows(&padded, layout).expect("tight rows");
        assert_eq!(tight.len(), 520);
        assert!(tight[..260].iter().all(|byte| *byte == 1));
        assert!(tight[260..].iter().all(|byte| *byte == 2));
    }

    #[test]
    fn timestamp_period_converts_ticks_to_milliseconds() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&10_u64.to_le_bytes());
        bytes.extend_from_slice(&510_u64.to_le_bytes());
        assert_eq!(timestamp_duration_ms(&bytes, 2.0).expect("duration"), 0.001);
    }

    #[test]
    fn surface_format_requires_srgb_and_preserves_preference_order() {
        assert_eq!(
            select_surface_format(&[
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ]),
            Some(wgpu::TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(
            select_surface_format(&[wgpu::TextureFormat::Rgba8Unorm]),
            None
        );
    }

    #[test]
    fn surface_presentation_prefers_fifo() {
        assert_eq!(
            select_present_mode(&[wgpu::PresentMode::Immediate, wgpu::PresentMode::Fifo]),
            Some(wgpu::PresentMode::Fifo)
        );
        assert_eq!(
            select_present_mode(&[wgpu::PresentMode::Immediate]),
            Some(wgpu::PresentMode::Immediate)
        );
        assert_eq!(select_present_mode(&[]), None);
    }
}
