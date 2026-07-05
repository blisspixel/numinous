//! Numinous GPU rendering.
//!
//! An adaptive `wgpu` context that picks whatever GPU the machine has (AMD,
//! NVIDIA, Intel, or Apple) across Vulkan, Metal, and DX12, and renders
//! offscreen with no window. This is how the same app "just works" on the dev
//! laptop's integrated AMD graphics, an RTX 4090, and a Mac mini, falling back to
//! a CPU adapter if no GPU is present. See `docs/ARCHITECTURE.md`.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// A ready-to-use GPU device and queue, chosen adaptively for this machine.
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_name: String,
    backend: String,
}

/// Uniform parameters for the fractal shader. Layout matches `mandelbrot.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    width: u32,
    height: u32,
    max_iter: u32,
    mode: u32,
    center_x: f32,
    center_y: f32,
    scale: f32,
    _pad1: f32,
    c_x: f32,
    c_y: f32,
    _pad2: f32,
    _pad3: f32,
}

/// Which escape-time fractal the shader computes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fractal {
    /// The Mandelbrot set: z starts at zero, c is the pixel.
    Mandelbrot,
    /// A Julia set: z starts at the pixel, c is this constant.
    Julia {
        /// The real part of the constant c.
        cx: f32,
        /// The imaginary part of the constant c.
        cy: f32,
    },
}

impl GpuContext {
    /// Create a context, adapting to whatever adapter this machine offers.
    ///
    /// Prefers a real GPU; falls back to a CPU adapter so it never fails to run.
    ///
    /// # Errors
    /// Returns an error string if no adapter at all can be acquired.
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .or_else(|_| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: true,
            }))
        })
        .map_err(|e| format!("no GPU or CPU adapter available: {e:?}"))?;

        let info = adapter.get_info();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|e| format!("failed to acquire device: {e:?}"))?;

        Ok(Self {
            device,
            queue,
            adapter_name: info.name,
            backend: format!("{:?}", info.backend),
        })
    }

    /// The human-readable name of the chosen adapter (for example the GPU model).
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    /// The graphics backend in use (for example `Vulkan`, `Metal`, or `Dx12`).
    #[must_use]
    pub fn backend(&self) -> &str {
        &self.backend
    }

    /// Render the Mandelbrot set to an RGBA byte buffer (`width * height * 4`).
    ///
    /// One-shot convenience over [`FractalRenderer`]; use the renderer directly
    /// for real-time frames.
    #[must_use]
    pub fn render_mandelbrot(
        self,
        width: u32,
        height: u32,
        center_x: f32,
        center_y: f32,
        scale: f32,
        max_iter: u32,
    ) -> Vec<u8> {
        let mut renderer = FractalRenderer::from_context(self);
        renderer.render(
            width,
            height,
            center_x,
            center_y,
            scale,
            max_iter,
            Fractal::Mandelbrot,
        )
    }
}

/// A persistent escape-time fractal renderer: the pipeline is built once, and
/// buffers are reused across frames (reallocated only when the size changes),
/// so it is fast enough to drive a window in real time.
pub struct FractalRenderer {
    context: GpuContext,
    pipeline: wgpu::ComputePipeline,
    /// Cached output and readback buffers with their pixel size.
    buffers: Option<(u32, u32, wgpu::Buffer, wgpu::Buffer)>,
}

impl FractalRenderer {
    /// Create a renderer on an adaptive context.
    ///
    /// # Errors
    /// Returns an error string if no adapter at all can be acquired.
    pub fn new() -> Result<Self, String> {
        Ok(Self::from_context(GpuContext::new()?))
    }

    /// Build the persistent pipeline on an existing context.
    #[must_use]
    pub fn from_context(context: GpuContext) -> Self {
        let module = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fractal"),
                source: wgpu::ShaderSource::Wgsl(include_str!("mandelbrot.wgsl").into()),
            });
        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("fractal"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        Self {
            context,
            pipeline,
            buffers: None,
        }
    }

    /// The adapter this renderer runs on.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        self.context.adapter_name()
    }

    /// Render one frame to an RGBA byte buffer (`width * height * 4`).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        width: u32,
        height: u32,
        center_x: f32,
        center_y: f32,
        scale: f32,
        max_iter: u32,
        fractal: Fractal,
    ) -> Vec<u8> {
        let (mode, c_x, c_y) = match fractal {
            Fractal::Mandelbrot => (0, 0.0, 0.0),
            Fractal::Julia { cx, cy } => (1, cx, cy),
        };
        let params = Params {
            width,
            height,
            max_iter,
            mode,
            center_x,
            center_y,
            scale,
            _pad1: 0.0,
            c_x,
            c_y,
            _pad2: 0.0,
            _pad3: 0.0,
        };
        let byte_len = u64::from(width) * u64::from(height) * 4;
        let device = &self.context.device;

        // Reuse the output buffers unless the frame size changed.
        let recreate = !matches!(&self.buffers, Some((w, h, _, _)) if *w == width && *h == height);
        if recreate {
            let storage = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fractal-output"),
                size: byte_len,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let read = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fractal-readback"),
                size: byte_len,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.buffers = Some((width, height, storage, read));
        }
        let Some((_, _, storage_buf, read_buf)) = &self.buffers else {
            return vec![0; byte_len as usize];
        };

        let param_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fractal-params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fractal-bind"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: param_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: storage_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("fractal"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        encoder.copy_buffer_to_buffer(storage_buf, 0, read_buf, 0, byte_len);
        self.context.queue.submit(Some(encoder.finish()));

        let slice = read_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let _ = rx.recv();

        let bytes = slice.get_mapped_range().to_vec();
        read_buf.unmap();
        bytes
    }
}
