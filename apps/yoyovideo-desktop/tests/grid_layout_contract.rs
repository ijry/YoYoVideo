use yoyovideo_desktop::{
    DEFAULT_ASPECT, GridCell, LogicalVideoRect, MAX_GRID_TILES, STRIP_HEIGHT, plan_grid,
};

const WIDE: LogicalVideoRect = LogicalVideoRect { x: 0.0, y: 0.0, width: 1200.0, height: 700.0 };

fn container(width: f32, height: f32) -> LogicalVideoRect {
    LogicalVideoRect { x: 0.0, y: 0.0, width, height }
}

fn aspects(n: usize) -> Vec<f32> {
    vec![DEFAULT_ASPECT; n]
}

/// How many distinct rows / columns the returned cells occupy.
fn shape(cells: &[GridCell]) -> (usize, usize) {
    let mut rows: Vec<f32> = cells.iter().map(|cell| cell.video.y).collect();
    let mut cols: Vec<f32> = cells.iter().map(|cell| cell.video.x).collect();
    for values in [&mut rows, &mut cols] {
        values.sort_by(|a, b| a.total_cmp(b));
        values.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    }
    (rows.len(), cols.len())
}

fn assert_inside(cell: &GridCell, container: LogicalVideoRect) {
    for rect in [cell.video, cell.strip] {
        assert!(rect.width >= 0.0 && rect.height >= 0.0, "no negative extents: {rect:?}");
        assert!(rect.x >= container.x - 0.5, "left edge inside: {rect:?}");
        assert!(rect.y >= container.y - 0.5, "top edge inside: {rect:?}");
        assert!(
            rect.x + rect.width <= container.x + container.width + 0.5,
            "right edge inside: {rect:?}"
        );
        assert!(
            rect.y + rect.height <= container.y + container.height + 0.5,
            "bottom edge inside: {rect:?}"
        );
    }
}

#[test]
fn no_tiles_produces_no_cells() {
    assert!(plan_grid(WIDE, &[], STRIP_HEIGHT, 8.0).is_empty());
}

#[test]
fn a_single_tile_uses_the_container_minus_its_strip() {
    let cells = plan_grid(WIDE, &aspects(1), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), 1);
    assert_inside(&cells[0], WIDE);
    // The strip sits directly below the picture and never on top of it.
    assert!(cells[0].strip.y >= cells[0].video.y + cells[0].video.height - 0.5);
    assert_eq!(cells[0].strip.height, STRIP_HEIGHT);
}

#[test]
fn two_wide_tiles_in_a_wide_container_go_side_by_side() {
    let cells = plan_grid(WIDE, &aspects(2), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), 2);
    assert_eq!(shape(&cells), (1, 2), "one row, two columns");
}

#[test]
fn four_tiles_form_two_by_two() {
    let cells = plan_grid(WIDE, &aspects(4), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), 4);
    assert_eq!(shape(&cells), (2, 2));
}

#[test]
fn nine_tiles_form_three_by_three() {
    let cells = plan_grid(WIDE, &aspects(9), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), 9);
    assert_eq!(shape(&cells), (3, 3));
}

#[test]
fn strips_never_overlap_any_picture() {
    // The control strip is Slint-drawn; the picture is a native child window that
    // composites above it. Any overlap would make the strip invisible and unclickable.
    let cells = plan_grid(WIDE, &aspects(6), STRIP_HEIGHT, 8.0);

    for (index, cell) in cells.iter().enumerate() {
        for (other_index, other) in cells.iter().enumerate() {
            let horizontal = cell.strip.x < other.video.x + other.video.width
                && other.video.x < cell.strip.x + cell.strip.width;
            let vertical = cell.strip.y < other.video.y + other.video.height
                && other.video.y < cell.strip.y + cell.strip.height;
            assert!(
                !(horizontal && vertical),
                "strip {index} overlaps picture {other_index}: {:?} vs {:?}",
                cell.strip,
                other.video
            );
        }
    }
}

#[test]
fn every_cell_stays_inside_the_container() {
    for count in 1..=MAX_GRID_TILES {
        let cells = plan_grid(WIDE, &aspects(count), STRIP_HEIGHT, 8.0);
        for cell in &cells {
            assert_inside(cell, WIDE);
        }
    }
}

#[test]
fn each_picture_keeps_its_own_aspect_ratio() {
    // A portrait clip beside landscape ones must letterbox, not stretch.
    let mixed = vec![DEFAULT_ASPECT, 9.0 / 16.0, 1.0];
    let cells = plan_grid(WIDE, &mixed, STRIP_HEIGHT, 8.0);

    for (cell, expected) in cells.iter().zip(&mixed) {
        let actual = cell.video.width / cell.video.height;
        assert!(
            (actual - expected).abs() < 0.02,
            "expected aspect {expected}, got {actual} from {:?}",
            cell.video
        );
    }
}

#[test]
fn more_than_the_maximum_is_truncated() {
    let cells = plan_grid(WIDE, &aspects(MAX_GRID_TILES + 4), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), MAX_GRID_TILES);
}

#[test]
fn a_degenerate_container_does_not_panic_or_go_negative() {
    for rect in [container(0.0, 0.0), container(10.0, 1.0), container(-5.0, -5.0)] {
        let cells = plan_grid(rect, &aspects(4), STRIP_HEIGHT, 8.0);
        for cell in &cells {
            assert!(cell.video.width >= 0.0 && cell.video.height >= 0.0, "{cell:?}");
            assert!(cell.strip.width >= 0.0 && cell.strip.height >= 0.0, "{cell:?}");
        }
    }
}

#[test]
fn the_container_offset_is_respected() {
    let offset = LogicalVideoRect { x: 40.0, y: 25.0, width: 800.0, height: 600.0 };
    let cells = plan_grid(offset, &aspects(4), STRIP_HEIGHT, 8.0);

    for cell in &cells {
        assert_inside(cell, offset);
    }
}

#[test]
fn the_layout_is_deterministic() {
    let first = plan_grid(WIDE, &aspects(5), STRIP_HEIGHT, 8.0);
    let second = plan_grid(WIDE, &aspects(5), STRIP_HEIGHT, 8.0);

    assert_eq!(first, second);
}
