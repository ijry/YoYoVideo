use yoyovideo_desktop::{
    DEFAULT_ASPECT, GridCell, LogicalVideoRect, MAX_GRID_TILES, MAX_TILE_SCALE, MIN_TILE_SCALE,
    STRIP_HEIGHT, clamp_tile_scale, plan_grid,
};

const WIDE: LogicalVideoRect = LogicalVideoRect { x: 0.0, y: 0.0, width: 1200.0, height: 700.0 };

fn container(width: f32, height: f32) -> LogicalVideoRect {
    LogicalVideoRect { x: 0.0, y: 0.0, width, height }
}

fn full(n: usize) -> Vec<f32> {
    vec![MAX_TILE_SCALE; n]
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
    assert!(plan_grid(WIDE, &[], &[], STRIP_HEIGHT, 8.0).is_empty());
}

#[test]
fn a_single_tile_uses_the_container_minus_its_strip() {
    let cells = plan_grid(WIDE, &aspects(1), &full(1), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), 1);
    assert_inside(&cells[0], WIDE);
    // The strip sits directly below the picture and never on top of it.
    assert!(cells[0].strip.y >= cells[0].video.y + cells[0].video.height - 0.5);
    assert_eq!(cells[0].strip.height, STRIP_HEIGHT);
}

#[test]
fn two_wide_tiles_in_a_wide_container_go_side_by_side() {
    let cells = plan_grid(WIDE, &aspects(2), &full(2), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), 2);
    assert_eq!(shape(&cells), (1, 2), "one row, two columns");
}

#[test]
fn four_tiles_form_two_by_two() {
    let cells = plan_grid(WIDE, &aspects(4), &full(4), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), 4);
    assert_eq!(shape(&cells), (2, 2));
}

#[test]
fn nine_tiles_form_three_by_three() {
    let cells = plan_grid(WIDE, &aspects(9), &full(9), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), 9);
    assert_eq!(shape(&cells), (3, 3));
}

#[test]
fn strips_never_overlap_any_picture() {
    // The control strip is Slint-drawn; the picture is a native child window that
    // composites above it. Any overlap would make the strip invisible and unclickable.
    let cells = plan_grid(WIDE, &aspects(6), &full(6), STRIP_HEIGHT, 8.0);

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
        let cells = plan_grid(WIDE, &aspects(count), &full(count), STRIP_HEIGHT, 8.0);
        for cell in &cells {
            assert_inside(cell, WIDE);
        }
    }
}

#[test]
fn each_picture_keeps_its_own_aspect_ratio() {
    // A portrait clip beside landscape ones must letterbox, not stretch.
    let mixed = vec![DEFAULT_ASPECT, 9.0 / 16.0, 1.0];
    let cells = plan_grid(WIDE, &mixed, &full(mixed.len()), STRIP_HEIGHT, 8.0);

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
    let cells = plan_grid(WIDE, &aspects(MAX_GRID_TILES + 4), &full(MAX_GRID_TILES + 4), STRIP_HEIGHT, 8.0);

    assert_eq!(cells.len(), MAX_GRID_TILES);
}

#[test]
fn a_degenerate_container_does_not_panic_or_go_negative() {
    for rect in [container(0.0, 0.0), container(10.0, 1.0), container(-5.0, -5.0)] {
        let cells = plan_grid(rect, &aspects(4), &full(4), STRIP_HEIGHT, 8.0);
        for cell in &cells {
            assert!(cell.video.width >= 0.0 && cell.video.height >= 0.0, "{cell:?}");
            assert!(cell.strip.width >= 0.0 && cell.strip.height >= 0.0, "{cell:?}");
        }
    }
}

#[test]
fn the_container_offset_is_respected() {
    let offset = LogicalVideoRect { x: 40.0, y: 25.0, width: 800.0, height: 600.0 };
    let cells = plan_grid(offset, &aspects(4), &full(4), STRIP_HEIGHT, 8.0);

    for cell in &cells {
        assert_inside(cell, offset);
    }
}

#[test]
fn the_layout_is_deterministic() {
    let first = plan_grid(WIDE, &aspects(5), &full(5), STRIP_HEIGHT, 8.0);
    let second = plan_grid(WIDE, &aspects(5), &full(5), STRIP_HEIGHT, 8.0);

    assert_eq!(first, second);
}

#[test]
fn a_tile_scale_shrinks_only_that_tile() {
    let full_size = plan_grid(WIDE, &aspects(4), &full(4), STRIP_HEIGHT, 8.0);
    let mut scales = full(4);
    scales[1] = 0.5;
    let scaled = plan_grid(WIDE, &aspects(4), &scales, STRIP_HEIGHT, 8.0);

    assert!(
        (scaled[1].video.width - full_size[1].video.width * 0.5).abs() < 0.5,
        "tile 1 halves: {:?}",
        scaled[1].video
    );
    for index in [0, 2, 3] {
        assert_eq!(
            scaled[index].video, full_size[index].video,
            "tile {index} must be untouched"
        );
    }
}

#[test]
fn a_scaled_tile_keeps_its_aspect_ratio_and_its_strip() {
    let mut scales = full(4);
    scales[0] = 0.6;
    let cells = plan_grid(WIDE, &aspects(4), &scales, STRIP_HEIGHT, 8.0);

    let ratio = cells[0].video.width / cells[0].video.height;
    assert!((ratio - DEFAULT_ASPECT).abs() < 0.02, "scaling must not distort: {ratio}");
    // The strip follows the picture down and matches its new width.
    assert!((cells[0].strip.y - (cells[0].video.y + cells[0].video.height)).abs() < 0.5);
    assert!((cells[0].strip.width - cells[0].video.width).abs() < 0.5);
}

#[test]
fn scaling_never_grows_a_tile_past_its_cell() {
    let mut scales = full(4);
    scales[0] = 4.0;
    let cells = plan_grid(WIDE, &aspects(4), &scales, STRIP_HEIGHT, 8.0);

    // Otherwise one tile would overlap its neighbours, and the strips with them.
    for cell in &cells {
        assert_inside(cell, WIDE);
    }
    assert_eq!(cells[0].video, plan_grid(WIDE, &aspects(4), &full(4), STRIP_HEIGHT, 8.0)[0].video);
}

#[test]
fn the_grid_shape_does_not_reflow_when_a_tile_shrinks() {
    // Shrinking must not rearrange the other tiles under the user's cursor, so the
    // shrunk tile has to stay within the cell it already occupied. (Row/column counting
    // cannot show this: a smaller tile re-centres, which moves its x.)
    let baseline = plan_grid(WIDE, &aspects(4), &full(4), STRIP_HEIGHT, 8.0);
    let mut scales = full(4);
    scales[0] = MIN_TILE_SCALE;
    let cells = plan_grid(WIDE, &aspects(4), &scales, STRIP_HEIGHT, 8.0);

    let before = baseline[0].video;
    let after = cells[0].video;
    assert!(after.width <= before.width + 0.5 && after.height <= before.height + 0.5);
    assert!(after.y >= before.y - 0.5, "must not creep above its cell");
    assert!(
        after.x >= before.x - 0.5 && after.x + after.width <= before.x + before.width + 0.5,
        "must stay within its original column: {after:?} vs {before:?}"
    );
    // And the tiles that were not touched keep their exact geometry.
    assert_eq!(cells[1].video, baseline[1].video);
    assert_eq!(cells[2].video, baseline[2].video);
    assert_eq!(cells[3].video, baseline[3].video);
}

#[test]
fn tile_scale_is_clamped_to_the_usable_range() {
    assert_eq!(clamp_tile_scale(5.0), MAX_TILE_SCALE);
    assert_eq!(clamp_tile_scale(0.0), MIN_TILE_SCALE);
    assert_eq!(clamp_tile_scale(-1.0), MIN_TILE_SCALE);
    assert_eq!(clamp_tile_scale(f32::NAN), MAX_TILE_SCALE, "a bad value must not shrink a tile");
    assert_eq!(clamp_tile_scale(0.5), 0.5);
}

#[test]
fn missing_scales_default_to_full_size() {
    let with_none = plan_grid(WIDE, &aspects(3), &[], STRIP_HEIGHT, 8.0);
    let with_full = plan_grid(WIDE, &aspects(3), &full(3), STRIP_HEIGHT, 8.0);

    assert_eq!(with_none, with_full);
}
