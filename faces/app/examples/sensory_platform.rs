//! Cross-platform correctness and physical pacing probe for App presentation.
//!
//! Portable mode proves that a deterministic, fully composed App room frame
//! reaches the feature-gated direct surface through the production presenter.
//! Physical mode adds a release-build, machine, adapter, sample, and p95
//! contract. See `docs/PERFORMANCE.md` for the claim boundary.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[path = "sensory_platform/mod.rs"]
mod probe_support;

#[allow(
    dead_code,
    reason = "the probe reuses the production HUD module but composes one representative state"
)]
#[path = "../src/hud.rs"]
mod hud;
#[path = "../src/input_feedback.rs"]
mod input_feedback;
#[allow(
    dead_code,
    reason = "the production input vocabulary has more states than this representative frame"
)]
#[path = "../src/input_legend.rs"]
mod input_legend;
#[path = "../src/presentation.rs"]
mod presentation;

use probe_support::contract::{Config, HELP, parse_args, validate_config};
use probe_support::evidence::{AdapterReceipt, SurfaceReceipt};
use probe_support::source::{SourceFrame, compose_source};

const MAX_SKIPPED_FRAMES: usize = 120;

struct Probe {
    config: Config,
    source: SourceFrame,
    window: Option<Arc<Window>>,
    presenter: Option<presentation::WindowPresenter>,
    adapter: Option<AdapterReceipt>,
    surface: Option<SurfaceReceipt>,
    warmups_completed: usize,
    presented_frames: usize,
    skipped_frames: usize,
    suboptimal_frames: usize,
    acquire_ms: Vec<f64>,
    render_and_present_ms: Vec<f64>,
    boundary_ms: Vec<f64>,
    failure: Option<String>,
}

impl Probe {
    fn new(config: Config, source: SourceFrame) -> Self {
        let capacity = config.samples;
        Self {
            config,
            source,
            window: None,
            presenter: None,
            adapter: None,
            surface: None,
            warmups_completed: 0,
            presented_frames: 0,
            skipped_frames: 0,
            suboptimal_frames: 0,
            acquire_ms: Vec::with_capacity(capacity),
            render_and_present_ms: Vec::with_capacity(capacity),
            boundary_ms: Vec::with_capacity(capacity),
            failure: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, reason: impl Into<String>) {
        if self.failure.is_none() {
            self.failure = Some(reason.into());
        }
        event_loop.exit();
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = &self.window else {
            self.fail(event_loop, "presentation window is unavailable");
            return;
        };
        let size = window.inner_size();
        let Some(presenter) = self.presenter.as_mut() else {
            self.fail(event_loop, "production presenter is unavailable");
            return;
        };
        let outcome = presenter.present(
            &self.source.rgba,
            self.config.width as usize,
            self.config.height as usize,
            size.width as usize,
            size.height as usize,
        );
        match outcome {
            Ok(presentation::PresentOutcome::Presented {
                gpu_frame: Some(frame),
            }) => {
                self.presented_frames += 1;
                self.suboptimal_frames += usize::from(frame.suboptimal);
                if self.warmups_completed < self.config.warmups {
                    self.warmups_completed += 1;
                } else {
                    self.acquire_ms
                        .push(probe_support::evidence::round_ms(frame.acquire_ms));
                    self.render_and_present_ms
                        .push(probe_support::evidence::round_ms(
                            frame.render_and_present_ms,
                        ));
                    self.boundary_ms
                        .push(probe_support::evidence::round_ms(frame.boundary_ms));
                }
                if self.boundary_ms.len() == self.config.samples {
                    event_loop.exit();
                } else {
                    self.request_redraw();
                }
            }
            Ok(presentation::PresentOutcome::Presented { gpu_frame: None }) => {
                self.fail(
                    event_loop,
                    "production presenter completed through software instead of the direct GPU surface",
                );
            }
            Ok(presentation::PresentOutcome::Skipped) => {
                self.skipped_frames += 1;
                if self.skipped_frames > MAX_SKIPPED_FRAMES {
                    self.fail(
                        event_loop,
                        "presentation surface remained unavailable for too many frames",
                    );
                } else {
                    self.request_redraw();
                }
            }
            Ok(presentation::PresentOutcome::FellBack(reason)) => self.fail(
                event_loop,
                format!("production presenter fell back to software: {reason}"),
            ),
            Err(error) => self.fail(event_loop, error),
        }
    }
}

impl ApplicationHandler for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            self.request_redraw();
            return;
        }
        let requested = PhysicalSize::new(self.config.width, self.config.height);
        let attributes = Window::default_attributes()
            .with_title("Numinous Sensory Lift platform proof")
            .with_inner_size(requested)
            .with_resizable(false);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(
                    event_loop,
                    format!("failed to create proof window: {error}"),
                );
                return;
            }
        };
        let size = window.inner_size();
        let presenter =
            match presentation::WindowPresenter::new(window.clone(), size.width, size.height) {
                Ok(presenter) => presenter,
                Err(error) => {
                    self.fail(
                        event_loop,
                        format!("failed to create production presenter: {error}"),
                    );
                    return;
                }
            };
        if let Some(info) = presenter.gpu_info() {
            self.adapter = Some(AdapterReceipt::from_info(&info));
            self.surface = Some(SurfaceReceipt::from_info(
                info,
                requested,
                window.inner_size(),
            ));
        }
        self.presenter = Some(presenter);
        self.window = Some(window);
        self.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => self.draw(event_loop),
            WindowEvent::CloseRequested => self.fail(
                event_loop,
                "proof window was closed before the requested samples completed",
            ),
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(config) = parse_args(std::env::args().skip(1))? else {
        print!("{HELP}");
        return Ok(());
    };
    let github_actions = std::env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true");
    validate_config(&config, !cfg!(debug_assertions), github_actions)?;
    if config.output.exists() {
        return Err(format!("receipt path already exists: {}", config.output.display()).into());
    }
    let source = compose_source(config.width, config.height)?;
    let binary_sha256 = probe_support::evidence::executable_sha256().ok();
    let mut probe = Probe::new(config, source);
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    if let Err(error) = event_loop.run_app(&mut probe) {
        probe.failure = Some(format!("platform event loop failed: {error}"));
    }

    let passed =
        probe_support::evidence::write_probe_receipt(&probe, binary_sha256, github_actions)?;
    println!("wrote {}", probe.config.output.display());
    if probe.config.check && !passed {
        return Err("Sensory Lift App platform proof failed; inspect the JSON receipt".into());
    }
    Ok(())
}
