//! Numinous GPU rendering.
//!
//! An adaptive `wgpu` context that picks whatever GPU the machine has (AMD,
//! NVIDIA, Intel, or Apple) across Vulkan, Metal, and DX12, and renders
//! offscreen with no window. This is how the same app "just works" on the dev
//! laptop's integrated AMD graphics, an RTX 4090, and a Mac mini, falling back to
//! a CPU adapter if no GPU is present. See `docs/ARCHITECTURE.md`.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Largest frame dimension accepted by the renderer.
pub const MAX_FRAME_DIMENSION: u32 = 4096;

const MAX_FRAME_BYTES: u64 = 64 * 1024 * 1024;
const WORKGROUP_SIZE: u32 = 8;

/// A recoverable failure while preparing, executing, or reading a GPU frame.
#[derive(Debug)]
pub enum RenderError {
    /// The requested dimensions are zero or exceed the product frame budget.
    InvalidDimensions {
        /// Requested frame width.
        width: u32,
        /// Requested frame height.
        height: u32,
    },
    /// The selected device cannot support this frame resource or dispatch.
    DeviceLimit(&'static str),
    /// Waiting for submitted work failed.
    Poll(wgpu::PollError),
    /// The asynchronous map callback could not report its result.
    MapCallbackDropped,
    /// WGPU reported that the readback buffer could not be mapped.
    Map(wgpu::BufferAsyncError),
    /// Host memory for the returned frame could not be reserved.
    HostAllocation,
    /// WGPU rejected or could not complete a frame operation.
    Device(wgpu::Error),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => {
                write!(
                    formatter,
                    "unsupported GPU frame dimensions {width}x{height}"
                )
            }
            Self::DeviceLimit(limit) => write!(formatter, "GPU frame exceeds {limit}"),
            Self::Poll(error) => write!(formatter, "GPU device poll failed: {error}"),
            Self::MapCallbackDropped => formatter.write_str("GPU map callback was dropped"),
            Self::Map(error) => write!(formatter, "GPU readback mapping failed: {error}"),
            Self::HostAllocation => formatter.write_str("GPU frame host allocation failed"),
            Self::Device(error) => write!(formatter, "GPU frame operation failed: {error}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Whether dimensions fit the renderer's product-wide frame budget.
#[must_use]
pub fn frame_size_supported(width: usize, height: usize) -> bool {
    let Ok(width) = u32::try_from(width) else {
        return false;
    };
    let Ok(height) = u32::try_from(height) else {
        return false;
    };
    width > 0
        && height > 0
        && width <= MAX_FRAME_DIMENSION
        && height <= MAX_FRAME_DIMENSION
        && u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .is_some_and(|bytes| bytes <= MAX_FRAME_BYTES)
}

#[derive(Debug, Clone, Copy)]
struct FrameLayout {
    byte_len: u64,
    byte_len_usize: usize,
    workgroups_x: u32,
    workgroups_y: u32,
}

impl FrameLayout {
    fn validate(width: u32, height: u32, limits: &wgpu::Limits) -> Result<Self, RenderError> {
        if !frame_size_supported(width as usize, height as usize) {
            return Err(RenderError::InvalidDimensions { width, height });
        }
        let byte_len = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(RenderError::InvalidDimensions { width, height })?;
        if byte_len > limits.max_storage_buffer_binding_size {
            return Err(RenderError::DeviceLimit(
                "maximum storage buffer binding size",
            ));
        }
        if byte_len > limits.max_buffer_size {
            return Err(RenderError::DeviceLimit("maximum buffer size"));
        }
        if limits.max_compute_workgroup_size_x < WORKGROUP_SIZE {
            return Err(RenderError::DeviceLimit("maximum compute workgroup width"));
        }
        if limits.max_compute_workgroup_size_y < WORKGROUP_SIZE {
            return Err(RenderError::DeviceLimit("maximum compute workgroup height"));
        }
        if limits.max_compute_invocations_per_workgroup < WORKGROUP_SIZE * WORKGROUP_SIZE {
            return Err(RenderError::DeviceLimit(
                "maximum compute invocations per workgroup",
            ));
        }
        let workgroups_x = width.div_ceil(WORKGROUP_SIZE);
        let workgroups_y = height.div_ceil(WORKGROUP_SIZE);
        if workgroups_x > limits.max_compute_workgroups_per_dimension
            || workgroups_y > limits.max_compute_workgroups_per_dimension
        {
            return Err(RenderError::DeviceLimit(
                "maximum compute workgroups per dimension",
            ));
        }
        let byte_len_usize = usize::try_from(byte_len)
            .map_err(|_| RenderError::InvalidDimensions { width, height })?;
        Ok(Self {
            byte_len,
            byte_len_usize,
            workgroups_x,
            workgroups_y,
        })
    }
}

fn copy_mapped_bytes(mapped: &[u8], byte_len: usize) -> Result<Vec<u8>, RenderError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(byte_len)
        .map_err(|_| RenderError::HostAllocation)?;
    bytes.extend_from_slice(mapped);
    Ok(bytes)
}

fn validate_map_completion(
    poll: Result<wgpu::PollStatus, wgpu::PollError>,
    completion: Result<Result<(), wgpu::BufferAsyncError>, std::sync::mpsc::RecvError>,
) -> Result<(), RenderError> {
    poll.map_err(RenderError::Poll)?;
    completion
        .map_err(|_| RenderError::MapCallbackDropped)?
        .map_err(RenderError::Map)
}

fn capture_device_errors<T>(
    device: &wgpu::Device,
    operation: impl FnOnce() -> Result<T, RenderError>,
) -> Result<T, RenderError> {
    let out_of_memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = operation();
    let validation_error = pollster::block_on(validation.pop());
    let internal_error = pollster::block_on(internal.pop());
    let out_of_memory_error = pollster::block_on(out_of_memory.pop());
    let captured = validation_error.or(internal_error).or(out_of_memory_error);
    match captured {
        Some(error) => Err(RenderError::Device(error)),
        None => result,
    }
}

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
    pub fn render_mandelbrot(
        self,
        width: u32,
        height: u32,
        center_x: f32,
        center_y: f32,
        scale: f32,
        max_iter: u32,
    ) -> Result<Vec<u8>, RenderError> {
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
    ) -> Result<Vec<u8>, RenderError> {
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
        let device = self.context.device.clone();
        let layout = FrameLayout::validate(width, height, &device.limits())?;
        capture_device_errors(&device, || self.render_frame(&device, layout, params))
    }

    fn render_frame(
        &mut self,
        device: &wgpu::Device,
        layout: FrameLayout,
        params: Params,
    ) -> Result<Vec<u8>, RenderError> {
        // Reuse the output buffers unless the frame size changed.
        let recreate = !matches!(&self.buffers, Some((w, h, _, _)) if *w == params.width && *h == params.height);
        if recreate {
            let storage = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fractal-output"),
                size: layout.byte_len,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let read = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fractal-readback"),
                size: layout.byte_len,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.buffers = Some((params.width, params.height, storage, read));
        }
        let Some((_, _, storage_buf, read_buf)) = &self.buffers else {
            return Err(RenderError::HostAllocation);
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
            pass.dispatch_workgroups(layout.workgroups_x, layout.workgroups_y, 1);
        }
        encoder.copy_buffer_to_buffer(storage_buf, 0, read_buf, 0, layout.byte_len);
        self.context.queue.submit(Some(encoder.finish()));

        let slice = read_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let map_result = match device.poll(wgpu::PollType::wait_indefinitely()) {
            Ok(poll) => validate_map_completion(Ok(poll), rx.recv()),
            Err(error) => Err(RenderError::Poll(error)),
        };
        if let Err(error) = map_result {
            read_buf.unmap();
            return Err(error);
        }

        let bytes = {
            let mapped = slice.get_mapped_range();
            copy_mapped_bytes(&mapped, layout.byte_len_usize)
        };
        read_buf.unmap();
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FrameLayout, RenderError, copy_mapped_bytes, frame_size_supported, validate_map_completion,
    };

    #[test]
    fn frame_layout_enforces_product_and_device_limits() {
        let limits = wgpu::Limits::default();
        let largest = FrameLayout::validate(4096, 4096, &limits).expect("largest frame");
        assert_eq!(largest.byte_len, 64 * 1024 * 1024);
        assert!(frame_size_supported(4096, 4096));
        assert!(!frame_size_supported(4097, 1));
        assert!(!frame_size_supported(0, 1));
        assert!(matches!(
            FrameLayout::validate(u32::MAX, u32::MAX, &limits),
            Err(RenderError::InvalidDimensions { .. })
        ));

        let mut small_binding = limits.clone();
        small_binding.max_storage_buffer_binding_size = 3;
        assert!(matches!(
            FrameLayout::validate(1, 1, &small_binding),
            Err(RenderError::DeviceLimit(_))
        ));

        let mut small_dispatch = limits;
        small_dispatch.max_compute_workgroups_per_dimension = 1;
        assert!(matches!(
            FrameLayout::validate(9, 1, &small_dispatch),
            Err(RenderError::DeviceLimit(_))
        ));

        for limits in [
            wgpu::Limits {
                max_compute_workgroup_size_x: 7,
                ..wgpu::Limits::default()
            },
            wgpu::Limits {
                max_compute_workgroup_size_y: 7,
                ..wgpu::Limits::default()
            },
            wgpu::Limits {
                max_compute_invocations_per_workgroup: 63,
                ..wgpu::Limits::default()
            },
        ] {
            assert!(matches!(
                FrameLayout::validate(1, 1, &limits),
                Err(RenderError::DeviceLimit(_))
            ));
        }
    }

    #[test]
    fn failed_map_completion_is_returned_before_mapped_access() {
        let (_, disconnected) = std::sync::mpsc::channel::<Result<(), wgpu::BufferAsyncError>>();
        assert!(matches!(
            validate_map_completion(Ok(wgpu::PollStatus::QueueEmpty), disconnected.recv()),
            Err(RenderError::MapCallbackDropped)
        ));
        assert!(matches!(
            validate_map_completion(
                Ok(wgpu::PollStatus::QueueEmpty),
                Ok(Err(wgpu::BufferAsyncError))
            ),
            Err(RenderError::Map(_))
        ));
        assert!(matches!(
            validate_map_completion(Err(wgpu::PollError::Timeout), Ok(Ok(()))),
            Err(RenderError::Poll(_))
        ));
        assert!(matches!(
            copy_mapped_bytes(&[], usize::MAX),
            Err(RenderError::HostAllocation)
        ));
    }
}
