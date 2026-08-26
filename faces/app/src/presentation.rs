use std::borrow::Cow;
use std::num::NonZeroU32;
use std::sync::Arc;

use winit::window::Window;

#[cfg(feature = "gpu-post")]
use numinous_gpu::{RenderError, SensorySurfaceRenderer};

const BACKGROUND_RGBA: [u8; 4] = [10, 11, 15, 255];

pub(crate) enum PresentOutcome {
    Presented,
    Skipped,
    #[cfg(feature = "gpu-post")]
    FellBack(String),
}

pub(crate) struct WindowPresenter {
    #[cfg(feature = "gpu-post")]
    window: Arc<Window>,
    #[cfg(feature = "gpu-post")]
    pending_fallback: Option<String>,
    backend: Backend,
}

enum Backend {
    Software(softbuffer::Surface<Arc<Window>, Arc<Window>>),
    #[cfg(feature = "gpu-post")]
    Gpu(Box<SensorySurfaceRenderer<'static>>),
}

#[cfg(feature = "gpu-post")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecoveryAction {
    Skip,
    Recreate,
    FallBack,
}

impl WindowPresenter {
    pub(crate) fn new(window: Arc<Window>, width: u32, height: u32) -> Result<Self, String> {
        #[cfg(feature = "gpu-post")]
        let (backend, pending_fallback) = match SensorySurfaceRenderer::new(
            window.clone(),
            width.max(1),
            height.max(1),
        ) {
            Ok(renderer) => (Backend::Gpu(Box::new(renderer)), None),
            Err(error) => {
                let backend = create_software_surface(window.clone()).map_err(|software| {
                    format!(
                        "GPU initialization failed ({error}); software initialization failed ({software})"
                    )
                })?;
                (backend, Some(error))
            }
        };
        #[cfg(not(feature = "gpu-post"))]
        let backend = create_software_surface(window.clone())?;
        #[cfg(not(feature = "gpu-post"))]
        let _ = (width, height);
        Ok(Self {
            #[cfg(feature = "gpu-post")]
            window,
            #[cfg(feature = "gpu-post")]
            pending_fallback,
            backend,
        })
    }

    pub(crate) fn present(
        &mut self,
        rgba: &[u8],
        raster_width: usize,
        raster_height: usize,
        window_width: usize,
        window_height: usize,
    ) -> Result<PresentOutcome, String> {
        let Some(width) = u32::try_from(window_width).ok().filter(|width| *width > 0) else {
            return Ok(PresentOutcome::Skipped);
        };
        let Some(height) = u32::try_from(window_height)
            .ok()
            .filter(|height| *height > 0)
        else {
            return Ok(PresentOutcome::Skipped);
        };
        let frame = fit_rgba(
            rgba,
            raster_width,
            raster_height,
            window_width,
            window_height,
        )?;

        #[cfg(feature = "gpu-post")]
        if matches!(self.backend, Backend::Gpu(_)) {
            return self.present_gpu(&frame, width, height);
        }

        present_software(&mut self.backend, &frame, width, height)?;
        #[cfg(feature = "gpu-post")]
        if let Some(reason) = self.pending_fallback.take() {
            return Ok(PresentOutcome::FellBack(reason));
        }
        Ok(PresentOutcome::Presented)
    }

    #[cfg(feature = "gpu-post")]
    fn present_gpu(
        &mut self,
        frame: &[u8],
        width: u32,
        height: u32,
    ) -> Result<PresentOutcome, String> {
        let result = match &mut self.backend {
            Backend::Gpu(renderer) => {
                if renderer.dimensions() != (width, height) {
                    match renderer.resize(width, height) {
                        Ok(()) => renderer.present_rgba(frame),
                        Err(error) => Err(error),
                    }
                } else {
                    renderer.present_rgba(frame)
                }
            }
            Backend::Software(_) => {
                return Err("GPU presenter entered an invalid state".to_string());
            }
        };
        match result {
            Ok(_) => Ok(PresentOutcome::Presented),
            Err(error) => match recovery_action(&error) {
                RecoveryAction::Skip => Ok(PresentOutcome::Skipped),
                RecoveryAction::Recreate => self.recreate_gpu_or_fallback(frame, width, height),
                RecoveryAction::FallBack => {
                    self.fallback_and_present(frame, width, height, error.to_string())
                }
            },
        }
    }

    #[cfg(feature = "gpu-post")]
    fn recreate_gpu_or_fallback(
        &mut self,
        frame: &[u8],
        width: u32,
        height: u32,
    ) -> Result<PresentOutcome, String> {
        match SensorySurfaceRenderer::new(self.window.clone(), width, height) {
            Ok(mut renderer) => match renderer.present_rgba(frame) {
                Ok(_) => {
                    self.backend = Backend::Gpu(Box::new(renderer));
                    Ok(PresentOutcome::Presented)
                }
                Err(error) => self.fallback_and_present(frame, width, height, error.to_string()),
            },
            Err(error) => self.fallback_and_present(frame, width, height, error),
        }
    }

    #[cfg(feature = "gpu-post")]
    fn fallback_and_present(
        &mut self,
        frame: &[u8],
        width: u32,
        height: u32,
        reason: String,
    ) -> Result<PresentOutcome, String> {
        let mut backend = create_software_surface(self.window.clone()).map_err(|software| {
            format!("GPU presentation failed ({reason}); software recovery failed ({software})")
        })?;
        present_software(&mut backend, frame, width, height).map_err(|software| {
            format!("GPU presentation failed ({reason}); software presentation failed ({software})")
        })?;
        self.backend = backend;
        Ok(PresentOutcome::FellBack(reason))
    }
}

#[cfg(feature = "gpu-post")]
fn recovery_action(error: &RenderError) -> RecoveryAction {
    match error {
        RenderError::SurfaceTimeout
        | RenderError::SurfaceOccluded
        | RenderError::SurfaceOutdated => RecoveryAction::Skip,
        RenderError::SurfaceLost => RecoveryAction::Recreate,
        _ => RecoveryAction::FallBack,
    }
}

fn create_software_surface(window: Arc<Window>) -> Result<Backend, String> {
    let context = softbuffer::Context::new(window.clone())
        .map_err(|error| format!("failed to create software display context: {error}"))?;
    let surface = softbuffer::Surface::new(&context, window)
        .map_err(|error| format!("failed to create software surface: {error}"))?;
    Ok(Backend::Software(surface))
}

fn present_software(
    backend: &mut Backend,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    #[cfg(feature = "gpu-post")]
    let surface = match backend {
        Backend::Software(surface) => surface,
        Backend::Gpu(_) => return Err("software presenter entered an invalid state".to_string()),
    };
    #[cfg(not(feature = "gpu-post"))]
    let Backend::Software(surface) = backend;
    present_software_surface(surface, rgba, width, height)
}

fn present_software_surface(
    surface: &mut softbuffer::Surface<Arc<Window>, Arc<Window>>,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    let width_nonzero = NonZeroU32::new(width).ok_or("software surface width is zero")?;
    let height_nonzero = NonZeroU32::new(height).ok_or("software surface height is zero")?;
    surface
        .resize(width_nonzero, height_nonzero)
        .map_err(|error| format!("failed to resize software surface: {error}"))?;
    let mut buffer = surface
        .buffer_mut()
        .map_err(|error| format!("failed to acquire software surface: {error}"))?;
    let expected_pixels = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or("software surface dimensions overflow")?;
    let expected_bytes = expected_pixels
        .checked_mul(4)
        .ok_or("software surface byte count overflows")?;
    if buffer.len() != expected_pixels || rgba.len() != expected_bytes {
        return Err(format!(
            "software surface size mismatch: {} pixels and {} RGBA bytes for {width}x{height}",
            buffer.len(),
            rgba.len()
        ));
    }
    for (pixel, channels) in buffer.iter_mut().zip(rgba.chunks_exact(4)) {
        *pixel = pack_rgb(channels);
    }
    buffer
        .present()
        .map_err(|error| format!("failed to present software surface: {error}"))
}

fn pack_rgb(channels: &[u8]) -> u32 {
    (u32::from(channels[0]) << 16) | (u32::from(channels[1]) << 8) | u32::from(channels[2])
}

fn fit_rgba<'a>(
    rgba: &'a [u8],
    raster_width: usize,
    raster_height: usize,
    window_width: usize,
    window_height: usize,
) -> Result<Cow<'a, [u8]>, String> {
    let raster_bytes = raster_width
        .checked_mul(raster_height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("raster dimensions overflow")?;
    if rgba.len() != raster_bytes {
        return Err(format!(
            "raster size mismatch: received {} RGBA bytes for {raster_width}x{raster_height}",
            rgba.len()
        ));
    }
    if raster_width == window_width && raster_height == window_height {
        return Ok(Cow::Borrowed(rgba));
    }
    let window_bytes = window_width
        .checked_mul(window_height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("window dimensions overflow")?;
    let mut fitted = Vec::new();
    fitted
        .try_reserve_exact(window_bytes)
        .map_err(|_| "window frame allocation failed")?;
    fitted.extend(BACKGROUND_RGBA.into_iter().cycle().take(window_bytes));
    let copy_width = raster_width.min(window_width);
    let copy_height = raster_height.min(window_height);
    for y in 0..copy_height {
        let source = y * raster_width * 4;
        let destination = y * window_width * 4;
        fitted[destination..destination + copy_width * 4]
            .copy_from_slice(&rgba[source..source + copy_width * 4]);
    }
    Ok(Cow::Owned(fitted))
}

#[cfg(test)]
mod tests {
    use super::{BACKGROUND_RGBA, fit_rgba, pack_rgb};

    #[test]
    fn exact_frame_is_borrowed() {
        let rgba = [1, 2, 3, 255, 4, 5, 6, 255];
        let fitted = fit_rgba(&rgba, 2, 1, 2, 1).expect("fit exact frame");
        assert!(matches!(fitted, std::borrow::Cow::Borrowed(_)));
        assert_eq!(fitted.as_ref(), rgba);
    }

    #[test]
    fn smaller_frame_is_padded_with_the_stage_background() {
        let rgba = [1, 2, 3, 255];
        let fitted = fit_rgba(&rgba, 1, 1, 2, 2).expect("fit padded frame");
        assert_eq!(
            fitted.as_ref(),
            [
                1,
                2,
                3,
                255,
                BACKGROUND_RGBA[0],
                BACKGROUND_RGBA[1],
                BACKGROUND_RGBA[2],
                BACKGROUND_RGBA[3],
                BACKGROUND_RGBA[0],
                BACKGROUND_RGBA[1],
                BACKGROUND_RGBA[2],
                BACKGROUND_RGBA[3],
                BACKGROUND_RGBA[0],
                BACKGROUND_RGBA[1],
                BACKGROUND_RGBA[2],
                BACKGROUND_RGBA[3],
            ]
        );
        assert_eq!(pack_rgb(&BACKGROUND_RGBA), 0x000A_0B0F);
    }

    #[test]
    fn malformed_or_overflowing_frames_are_rejected() {
        assert!(fit_rgba(&[0; 3], 1, 1, 1, 1).is_err());
        assert!(fit_rgba(&[], usize::MAX, 2, 1, 1).is_err());
        assert!(fit_rgba(&[], 0, 0, usize::MAX, 2).is_err());
    }

    #[cfg(feature = "gpu-post")]
    #[test]
    fn surface_failures_have_explicit_recovery_actions() {
        use numinous_gpu::RenderError;

        use super::{RecoveryAction, recovery_action};

        for error in [
            RenderError::SurfaceTimeout,
            RenderError::SurfaceOccluded,
            RenderError::SurfaceOutdated,
        ] {
            assert_eq!(recovery_action(&error), RecoveryAction::Skip);
        }
        assert_eq!(
            recovery_action(&RenderError::SurfaceLost),
            RecoveryAction::Recreate
        );
        assert_eq!(
            recovery_action(&RenderError::InvalidDimensions {
                width: 0,
                height: 1,
            }),
            RecoveryAction::FallBack
        );
        assert_eq!(
            recovery_action(&RenderError::SurfaceValidation),
            RecoveryAction::FallBack
        );
    }
}
