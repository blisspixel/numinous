//! Built-in rooms. Each module implements the [`crate::room::Room`] contract.

/// Shared escape-time budget for the CPU and accelerated Mandelbrot and Julia
/// renderers. Keeping one budget prevents interaction from changing detail
/// merely because a machine has a compatible GPU.
pub const FRACTAL_MAX_ITER: u32 = 160;

#[macro_use]
mod catalog;

macro_rules! declare_room_modules {
    ($(($module:ident, $room:ident, $metadata:expr)),* $(,)?) => {
        $(pub mod $module;)*
    };
}

catalog_rooms!(declare_room_modules);
hidden_rooms!(declare_room_modules);

#[cfg(test)]
pub(crate) use catalog::ROOM_OWN_ANSWER;
#[cfg(test)]
pub(crate) use catalog::ROOM_SOURCE_IDS;
pub use catalog::{ROOM_CATALOG, canonical_room_id, catalog_index, room_meta_by_id};
pub(crate) use catalog::{construct_all, construct_by_id, construct_hidden_by_id};

pub mod buffon_aha;
pub mod galton_aha;
pub mod kepler_aha;
pub mod nontransitive_aha;
pub mod parrondo_aha;
pub mod pendulum_aha;
pub mod times_tables_aha;
/// Select a deterministic variation stream for replayable room novelty.
///
/// `stream_id` separates visual streams. It is not cryptographic material.
pub(super) fn variation_unit(seed: u64, stream_id: u64) -> f64 {
    if seed == 0 {
        0.0
    } else {
        let mut rng = crate::rng::SplitMix64::new(seed ^ stream_id);
        rng.next_f64()
    }
}

pub(super) fn variation_signed(seed: u64, stream_id: u64) -> f64 {
    if seed == 0 {
        0.0
    } else {
        variation_unit(seed, stream_id) * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::{variation_signed, variation_unit};

    #[test]
    fn variation_streams_are_canonical_replayable_and_distinct() {
        assert_eq!(variation_unit(0, 11), 0.0);
        assert_eq!(variation_signed(0, 11), 0.0);

        let first = variation_unit(42, 11);
        assert_eq!(first, variation_unit(42, 11));
        assert_ne!(first, variation_unit(42, 12));
        assert_eq!(variation_signed(42, 11), first * 2.0 - 1.0);
    }
}
