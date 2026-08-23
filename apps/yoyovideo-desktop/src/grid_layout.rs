use crate::LogicalVideoRect;

/// Hardware decode contexts are finite, so the grid is capped rather than unbounded.
pub const MAX_GRID_TILES: usize = 9;
/// Assumed aspect ratio until mpv reports the real picture size.
pub const DEFAULT_ASPECT: f32 = 16.0 / 9.0;
/// Height of the per-tile control strip.
pub const STRIP_HEIGHT: f32 = 34.0;

/// Largest grid edge. `MAX_GRID_TILES` is 9, so 3x3 covers every case.
const MAX_EDGE: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl TileRect {
    const EMPTY: Self = Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
}

/// One tile: where its picture goes, and where its control strip goes.
///
/// `video` is handed to a native child window; `strip` is drawn by Slint. They never
/// overlap, because the native surface composites above the Slint canvas and would hide
/// a strip drawn underneath it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridCell {
    pub video: TileRect,
    pub strip: TileRect,
}

/// Arranges up to [`MAX_GRID_TILES`] tiles inside `container`.
///
/// Each picture is scaled to fit its cell while keeping its own aspect ratio and is
/// centred horizontally, so mixing portrait and landscape clips letterboxes instead of
/// stretching. Tiles beyond the cap are dropped.
///
/// Returns one [`GridCell`] per tile, in input order.
pub fn plan_grid(
    container: LogicalVideoRect,
    aspects: &[f32],
    strip_height: f32,
    gutter: f32,
) -> Vec<GridCell> {
    let count = aspects.len().min(MAX_GRID_TILES);
    if count == 0 {
        return Vec::new();
    }

    let width = container.width.max(0.0);
    let height = container.height.max(0.0);
    let strip_height = strip_height.max(0.0);
    let gutter = gutter.max(0.0);

    let (columns, rows) = best_shape(count, width, height, aspects, strip_height, gutter);

    // Split the container evenly, then let each tile letterbox inside its own cell.
    let cell_width = ((width - gutter * (columns.saturating_sub(1)) as f32) / columns as f32).max(0.0);
    let cell_height = ((height - gutter * (rows.saturating_sub(1)) as f32) / rows as f32).max(0.0);

    (0..count)
        .map(|index| {
            let column = index % columns;
            let row = index / columns;
            let cell_x = container.x + column as f32 * (cell_width + gutter);
            let cell_y = container.y + row as f32 * (cell_height + gutter);
            let aspect = sane_aspect(aspects[index]);
            cell(cell_x, cell_y, cell_width, cell_height, aspect, strip_height)
        })
        .collect()
}

/// Picks the grid shape that shows the most picture area.
///
/// Brute force is fine at this size: at most 9 tiles means at most 9 candidate shapes.
/// Ties break toward fewer rows, then fewer columns, so the result is deterministic.
fn best_shape(
    count: usize,
    width: f32,
    height: f32,
    aspects: &[f32],
    strip_height: f32,
    gutter: f32,
) -> (usize, usize) {
    let mut best = (count.min(MAX_EDGE).max(1), 1);
    let mut best_area = -1.0_f32;

    for columns in 1..=MAX_EDGE {
        let rows = count.div_ceil(columns);
        if rows > MAX_EDGE {
            continue;
        }

        let cell_width =
            ((width - gutter * (columns.saturating_sub(1)) as f32) / columns as f32).max(0.0);
        let cell_height =
            ((height - gutter * (rows.saturating_sub(1)) as f32) / rows as f32).max(0.0);

        let area: f32 = (0..count)
            .map(|index| {
                let video = cell(0.0, 0.0, cell_width, cell_height, sane_aspect(aspects[index]), strip_height).video;
                video.width * video.height
            })
            .sum();

        // `>` keeps the first (fewest-rows) candidate on a tie.
        if area > best_area {
            best_area = area;
            best = (columns, rows);
        }
    }

    best
}

/// Fits one picture plus its strip into a cell.
fn cell(
    cell_x: f32,
    cell_y: f32,
    cell_width: f32,
    cell_height: f32,
    aspect: f32,
    strip_height: f32,
) -> GridCell {
    // The strip is only worth reserving if something is left for the picture.
    let strip_height = if cell_height > strip_height { strip_height } else { 0.0 };
    let picture_box = (cell_height - strip_height).max(0.0);
    if cell_width <= 0.0 || picture_box <= 0.0 {
        return GridCell { video: TileRect::EMPTY, strip: TileRect::EMPTY };
    }

    // Letterbox: shrink whichever axis would overflow.
    let mut video_width = cell_width;
    let mut video_height = video_width / aspect;
    if video_height > picture_box {
        video_height = picture_box;
        video_width = video_height * aspect;
    }

    let video_x = cell_x + (cell_width - video_width) / 2.0;
    let video = TileRect { x: video_x, y: cell_y, width: video_width, height: video_height };
    let strip = TileRect {
        x: video_x,
        y: cell_y + video_height,
        width: video_width,
        height: strip_height,
    };

    GridCell { video, strip }
}

/// Guards against zero, negative, and non-finite ratios reaching the division.
fn sane_aspect(aspect: f32) -> f32 {
    if aspect.is_finite() && aspect > 0.0 { aspect.clamp(0.1, 10.0) } else { DEFAULT_ASPECT }
}
