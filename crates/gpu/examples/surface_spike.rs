//! Measures the Sensory Lift pipeline through a real window surface.

use numinous_gpu::{RenderError, SensorySurfaceRenderer};
use serde::Serialize;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[path = "post_spike/source.rs"]
mod source;

use source::source_frame;

const DEFAULT_WIDTH: u32 = 1920;
const DEFAULT_HEIGHT: u32 = 1080;
const DEFAULT_WARMUPS: usize = 30;
const DEFAULT_SAMPLES: usize = 120;
const MAX_TRANSIENT_FRAMES: usize = 120;

struct Arguments {
    width: u32,
    height: u32,
    warmups: usize,
    samples: usize,
    budget_ms: f64,
    check: bool,
}

struct Benchmark {
    arguments: Arguments,
    window: Option<Arc<Window>>,
    renderer: Option<SensorySurfaceRenderer<'static>>,
    source: Vec<u8>,
    warmups_completed: usize,
    acquire_ms: Vec<f64>,
    render_and_present_ms: Vec<f64>,
    boundary_ms: Vec<f64>,
    suboptimal_frames: usize,
    transient_frames: usize,
    adapter: String,
    backend: String,
    surface_format: String,
    present_mode: String,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Receipt<'a> {
    schema: &'a str,
    schema_version: u32,
    boundary: &'a str,
    adapter: &'a str,
    backend: &'a str,
    surface_format: &'a str,
    present_mode: &'a str,
    desired_maximum_frame_latency: u32,
    width: u32,
    height: u32,
    warmups: usize,
    samples: usize,
    transient_frames: usize,
    suboptimal_frames: usize,
    acquire_ms: Samples,
    render_and_present_ms: Samples,
    boundary_ms: Samples,
    boundary_budget_ms: f64,
    verdict: &'static str,
}

#[derive(Serialize)]
struct Samples {
    raw: Vec<f64>,
    p50: f64,
    p95: f64,
    maximum: f64,
}

impl Samples {
    fn from_raw(raw: Vec<f64>) -> Self {
        let mut ordered = raw.clone();
        ordered.sort_by(f64::total_cmp);
        Self {
            p50: percentile(&ordered, 0.50),
            p95: percentile(&ordered, 0.95),
            maximum: ordered.last().copied().unwrap_or_default(),
            raw,
        }
    }
}

impl Benchmark {
    fn new(arguments: Arguments) -> Self {
        let source = source_frame(arguments.width, arguments.height);
        let sample_capacity = arguments.samples;
        Self {
            arguments,
            window: None,
            renderer: None,
            source,
            warmups_completed: 0,
            acquire_ms: Vec::with_capacity(sample_capacity),
            render_and_present_ms: Vec::with_capacity(sample_capacity),
            boundary_ms: Vec::with_capacity(sample_capacity),
            suboptimal_frames: 0,
            transient_frames: 0,
            adapter: String::new(),
            backend: String::new(),
            surface_format: String::new(),
            present_mode: String::new(),
            error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl Into<String>) {
        self.error = Some(error.into());
        event_loop.exit();
    }

    fn request_next_frame(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn handle_surface_error(&mut self, event_loop: &ActiveEventLoop, error: RenderError) {
        match error {
            RenderError::SurfaceTimeout
            | RenderError::SurfaceOccluded
            | RenderError::SurfaceOutdated => {
                self.transient_frames += 1;
                if self.transient_frames > MAX_TRANSIENT_FRAMES {
                    self.fail(
                        event_loop,
                        "presentation surface remained unavailable for too many frames",
                    );
                } else {
                    self.request_next_frame();
                }
            }
            other => self.fail(event_loop, other.to_string()),
        }
    }

    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        let result = self
            .renderer
            .as_mut()
            .expect("renderer exists after resume")
            .present_rgba(&self.source);
        let frame = match result {
            Ok(frame) => frame,
            Err(error) => {
                self.handle_surface_error(event_loop, error);
                return;
            }
        };
        if self.warmups_completed < self.arguments.warmups {
            self.warmups_completed += 1;
            self.request_next_frame();
            return;
        }
        self.acquire_ms.push(round_ms(frame.acquire_ms));
        self.render_and_present_ms
            .push(round_ms(frame.render_and_present_ms));
        self.boundary_ms.push(round_ms(frame.boundary_ms));
        self.suboptimal_frames += usize::from(frame.suboptimal);
        if self.boundary_ms.len() == self.arguments.samples {
            event_loop.exit();
        } else {
            self.request_next_frame();
        }
    }
}

impl ApplicationHandler for Benchmark {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            self.request_next_frame();
            return;
        }
        let requested_size = PhysicalSize::new(self.arguments.width, self.arguments.height);
        let attributes = Window::default_attributes()
            .with_title("Numinous Sensory Lift surface measurement")
            .with_inner_size(requested_size)
            .with_resizable(false);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(
                    event_loop,
                    format!("failed to create measurement window: {error}"),
                );
                return;
            }
        };
        let actual_size = window.inner_size();
        if actual_size != requested_size {
            self.fail(
                event_loop,
                format!(
                    "window manager granted {}x{}, requested {}x{}",
                    actual_size.width,
                    actual_size.height,
                    requested_size.width,
                    requested_size.height
                ),
            );
            return;
        }
        let renderer = match SensorySurfaceRenderer::new(
            window.clone(),
            self.arguments.width,
            self.arguments.height,
        ) {
            Ok(renderer) => renderer,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        self.adapter = renderer.adapter_name().to_owned();
        self.backend = renderer.backend().to_owned();
        self.surface_format = format!("{:?}", renderer.surface_format());
        self.present_mode = format!("{:?}", renderer.present_mode());
        self.renderer = Some(renderer);
        self.window = Some(window);
        self.request_next_frame();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => self.draw(event_loop),
            WindowEvent::CloseRequested => {
                self.fail(
                    event_loop,
                    "measurement window was closed before completion",
                );
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = arguments()?;
    let mut benchmark = Benchmark::new(arguments);
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut benchmark)?;
    if let Some(error) = benchmark.error {
        return Err(error.into());
    }
    let acquire_ms = Samples::from_raw(benchmark.acquire_ms);
    let render_and_present_ms = Samples::from_raw(benchmark.render_and_present_ms);
    let boundary_ms = Samples::from_raw(benchmark.boundary_ms);
    let passed = boundary_ms.p95 <= benchmark.arguments.budget_ms;
    let receipt = Receipt {
        schema: "numinous.sensory-surface-spike",
        schema_version: 1,
        boundary: "host time from swapchain acquire request through sRGB upload, five render passes, queue submission, and queue presentation request; excludes compositor and display scanout",
        adapter: &benchmark.adapter,
        backend: &benchmark.backend,
        surface_format: &benchmark.surface_format,
        present_mode: &benchmark.present_mode,
        desired_maximum_frame_latency: 1,
        width: benchmark.arguments.width,
        height: benchmark.arguments.height,
        warmups: benchmark.arguments.warmups,
        samples: benchmark.arguments.samples,
        transient_frames: benchmark.transient_frames,
        suboptimal_frames: benchmark.suboptimal_frames,
        acquire_ms,
        render_and_present_ms,
        boundary_ms,
        boundary_budget_ms: benchmark.arguments.budget_ms,
        verdict: if passed { "pass" } else { "fail" },
    };
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    if benchmark.arguments.check && !passed {
        return Err("Sensory Lift surface boundary exceeded its budget".into());
    }
    Ok(())
}

fn arguments() -> Result<Arguments, Box<dyn std::error::Error>> {
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
    let mut warmups = DEFAULT_WARMUPS;
    let mut samples = DEFAULT_SAMPLES;
    let mut budget_ms = 33.0;
    let mut check = false;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--width" => width = positive_u32(args.next(), "--width")?,
            "--height" => height = positive_u32(args.next(), "--height")?,
            "--warmups" => warmups = positive_usize(args.next(), "--warmups")?,
            "--samples" => samples = positive_usize(args.next(), "--samples")?,
            "--budget-ms" => budget_ms = positive_f64(args.next(), "--budget-ms")?,
            "--check" => check = true,
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }
    Ok(Arguments {
        width,
        height,
        warmups,
        samples,
        budget_ms,
        check,
    })
}

fn positive_u32(value: Option<String>, option: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let value = value.ok_or_else(|| format!("{option} requires a value"))?;
    let parsed = value.parse::<u32>()?;
    if parsed == 0 {
        return Err(format!("{option} must be positive").into());
    }
    Ok(parsed)
}

fn positive_usize(
    value: Option<String>,
    option: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let value = value.ok_or_else(|| format!("{option} requires a value"))?;
    let parsed = value.parse::<usize>()?;
    if parsed == 0 {
        return Err(format!("{option} must be positive").into());
    }
    Ok(parsed)
}

fn positive_f64(value: Option<String>, option: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let value = value.ok_or_else(|| format!("{option} requires a value"))?;
    let parsed = value.parse::<f64>()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("{option} must be a positive finite number").into());
    }
    Ok(parsed)
}

fn percentile(ordered: &[f64], quantile: f64) -> f64 {
    let rank = (quantile * ordered.len() as f64).ceil() as usize;
    ordered[rank.saturating_sub(1).min(ordered.len().saturating_sub(1))]
}

fn round_ms(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::{Samples, percentile, positive_f64, positive_u32, positive_usize};

    #[test]
    fn sample_summary_uses_nearest_rank() {
        let summary = Samples::from_raw(vec![4.0, 1.0, 3.0, 2.0]);
        assert_eq!(summary.p50, 2.0);
        assert_eq!(summary.p95, 4.0);
        assert_eq!(percentile(&[1.0], 0.95), 1.0);
    }

    #[test]
    fn numeric_options_reject_zero_and_non_finite_values() {
        assert!(positive_u32(Some("0".to_owned()), "--width").is_err());
        assert!(positive_usize(Some("0".to_owned()), "--samples").is_err());
        assert!(positive_f64(Some("NaN".to_owned()), "--budget-ms").is_err());
    }
}
