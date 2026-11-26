//! Specular reflection on an ellipse with horizontal semiaxis one.

use super::phase_plane::PhasePlane;
use crate::surface::Surface;

#[derive(Clone, Copy, Debug)]
pub(super) struct Ellipse {
    pub(super) minor: f64,
}

/// A point on the rim and the unit direction leaving it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Ray {
    pub(super) point: (f64, f64),
    direction: (f64, f64),
}

impl Ellipse {
    pub(super) fn rim(self, phase: f64) -> (f64, f64) {
        (phase.cos(), self.minor * phase.sin())
    }

    /// Both tables use one physical scale for the two axes, including on text surfaces.
    pub(super) fn draw_rim(self, surface: &mut dyn Surface, mark: char) -> Option<PhasePlane> {
        let bounds = [(-1.0, -self.minor), (1.0, self.minor)];
        let plate = PhasePlane::fit(surface, bounds.into_iter())?;
        let mut prev = plate.point(1.0, 0.0);
        for step in 1..=128 {
            let (x, y) = self.rim(std::f64::consts::TAU * step as f64 / 128.0);
            let next = plate.point(x, y);
            surface.line(prev.0, prev.1, next.0, next.1, mark);
            prev = next;
        }
        Some(plate)
    }

    /// Aim from the inward normal toward the counterclockwise tangent.
    pub(super) fn launch(self, phase: f64, aim: f64) -> Ray {
        let point = self.rim(phase);
        let nx = point.0;
        let ny = point.1 / self.minor.powi(2);
        let length = nx.hypot(ny);
        let (sin, cos) = aim.sin_cos();
        Ray {
            point,
            direction: (
                (-nx * cos - ny * sin) / length,
                (-ny * cos + nx * sin) / length,
            ),
        }
    }

    pub(super) fn bounce(self, ray: Ray) -> Ray {
        let (x, y) = ray.point;
        let (vx, vy) = ray.direction;
        let b2 = self.minor.powi(2);
        // The ray starts on the rim, so the intersection quadratic already
        // has root zero. Its other root is the flight to the opposite wall.
        let distance = -2.0 * (x * vx + y * vy / b2) / (vx * vx + vy * vy / b2);
        let point = (x + distance * vx, y + distance * vy);
        let (nx, ny) = (point.0, point.1 / b2);
        let normal_component = (vx * nx + vy * ny) / (nx * nx + ny * ny);
        Ray {
            point,
            direction: (
                vx - 2.0 * normal_component * nx,
                vy - 2.0 * normal_component * ny,
            ),
        }
    }

    /// Confocal parameter of the line through the ray, in squared table units.
    pub(super) fn caustic_parameter(self, ray: Ray) -> f64 {
        let (x, y) = ray.point;
        let (vx, vy) = ray.direction;
        vy * vy + (self.minor * vx).powi(2) - (x * vy - y * vx).powi(2)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_6, TAU};

    use super::{Ellipse, Ray};

    #[test]
    fn an_axial_ray_alternates_wall_hits() {
        let table = Ellipse { minor: 0.65 };
        let mut ray = table.launch(0.0, 0.0);
        for expected_x in [-1.0, 1.0, -1.0, 1.0] {
            ray = table.bounce(ray);
            assert_eq!(ray.point, (expected_x, 0.0));
        }
    }

    #[test]
    fn the_focal_ray_leaves_for_the_other_focus() {
        let table = Ellipse { minor: 0.65 };
        let c = (1.0 - table.minor.powi(2)).sqrt();
        // From the top of the table the distance to either focus is one.
        let ray = Ray {
            point: (0.0, table.minor),
            direction: (c, -table.minor),
        };
        let reflected = table.bounce(ray);
        let (vx, vy) = reflected.direction;
        let (x, y) = reflected.point;
        let to_focus = (-c - x, -y);
        assert!((to_focus.0 * vy - to_focus.1 * vx).abs() < 1e-12);
        assert!(to_focus.0 * vx + to_focus.1 * vy > 0.0);
        assert!((table.caustic_parameter(ray) - table.minor.powi(2)).abs() < 1e-12);
    }

    #[test]
    fn rim_and_confocal_parameter_survive_repeated_reflections() {
        for minor in [0.62, 0.65, 0.73] {
            let table = Ellipse { minor };
            for phase in [0.0, TAU / 7.0, FRAC_PI_2] {
                for slope in [FRAC_PI_6, FRAC_PI_4, 1.2] {
                    let mut ray = table.launch(phase, FRAC_PI_2 - slope);
                    let parameter = table.caustic_parameter(ray);
                    if phase == FRAC_PI_2 {
                        // At the top, the tangent is horizontal; lambda = sin(slope)^2.
                        assert!((parameter - slope.sin().powi(2)).abs() < 1e-12);
                    }
                    for _ in 0..80 {
                        let next = table.bounce(ray);
                        let (x, y) = next.point;
                        assert!((x * x + (y / minor).powi(2) - 1.0).abs() < 1e-11);
                        assert!((x - ray.point.0).hypot(y - ray.point.1) > 0.01);
                        assert!((table.caustic_parameter(next) - parameter).abs() < 1e-11);
                        ray = next;
                    }
                }
            }
        }
    }
}
