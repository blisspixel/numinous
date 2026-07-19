//! Shared bounded rendering for live and replayed Nim boards.

use numinous_core::{Raster, Surface};

const HEAP_COUNT: usize = 3;
const MAX_HEAP_STONES: u32 = 7;

/// One live heap selection drawn above the immutable board state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NimSelection {
    /// Zero-based selected heap.
    pub heap: usize,
    /// Number of trailing stones highlighted for removal.
    pub take: u32,
}

/// Returns the scale shared by the Nim board and its surrounding App copy.
#[must_use]
pub fn nim_scale(width: usize) -> i32 {
    (i32::try_from(width).unwrap_or(i32::MAX) / 400).clamp(1, 3)
}

/// Draws one validated three-heap Nim board.
///
/// The renderer accepts only states reachable from the core game. This keeps a
/// public replay caller from turning untrusted heap counts into unbounded draw
/// work. Live selection is optional because a read-only viewer has no move
/// cursor.
#[must_use]
pub fn draw_nim_board(
    heaps: &[u32],
    selection: Option<NimSelection>,
    width: usize,
    height: usize,
) -> Option<Raster> {
    if heaps.len() != HEAP_COUNT
        || heaps.iter().any(|count| *count > MAX_HEAP_STONES)
        || selection.is_some_and(|selected| {
            selected.heap >= HEAP_COUNT
                || selected.take == 0
                || selected.take > heaps[selected.heap]
        })
    {
        return None;
    }

    let mut raster = Raster::with_accent(width, height, [230, 200, 120]);
    let width = raster.width();
    let height = raster.height();
    let scale = nim_scale(width);
    raster.dim_rows(0, 12 + 7 * scale, 40);
    raster.dim_rows(height as i32 - 38 * scale, height as i32, 40);
    numinous_core::draw_text(&mut raster, "NIM: LAST STONE WINS", 10, 10, scale, '#');
    let top = 20 * scale + 10;
    let row_height = (height as i32 - top - 42 * scale) / HEAP_COUNT as i32;
    let stone = (row_height / 2).clamp(4, 10 * scale);
    for (heap, &count) in heaps.iter().enumerate() {
        let y = top + heap as i32 * row_height + row_height / 2;
        let selected = selection.filter(|selected| selected.heap == heap);
        numinous_core::draw_text(
            &mut raster,
            &format!("{}{}", if selected.is_some() { ">" } else { " " }, heap + 1),
            10,
            y - 4 * scale,
            scale,
            if selected.is_some() { '#' } else { '*' },
        );
        for index in 0..count {
            let x = 40 + index as i32 * (stone + 6);
            let aimed =
                selected.is_some_and(|selected| index >= count.saturating_sub(selected.take));
            let mark = if aimed { '#' } else { '*' };
            for offset in 0..stone {
                raster.line(x, y + offset, x + stone, y + offset, mark);
            }
        }
    }
    Some(raster)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_rejects_unreachable_or_unbounded_state() {
        assert!(draw_nim_board(&[1, 2], None, 320, 180).is_none());
        assert!(draw_nim_board(&[1, 2, 8], None, 320, 180).is_none());
        assert!(
            draw_nim_board(
                &[1, 2, 3],
                Some(NimSelection { heap: 3, take: 1 }),
                320,
                180,
            )
            .is_none()
        );
        assert!(
            draw_nim_board(
                &[1, 2, 3],
                Some(NimSelection { heap: 1, take: 3 }),
                320,
                180,
            )
            .is_none()
        );
        let hostile = draw_nim_board(&[1, 2, 3], None, usize::MAX, 0)
            .expect("hostile dimensions remain Raster-bounded");
        assert_eq!((hostile.width(), hostile.height()), (4096, 0));
        assert_eq!(nim_scale(usize::MAX), 3);
    }

    #[test]
    fn live_selection_changes_only_a_valid_bounded_board() {
        let plain = draw_nim_board(&[3, 5, 7], None, 360, 240)
            .expect("plain board")
            .to_rgba();
        let selected = draw_nim_board(
            &[3, 5, 7],
            Some(NimSelection { heap: 1, take: 2 }),
            360,
            240,
        )
        .expect("selected board")
        .to_rgba();
        assert_ne!(plain, selected);
        assert_eq!(plain.len(), selected.len());
    }
}
