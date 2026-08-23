use std::path::PathBuf;

use tempfile::NamedTempFile;
use yoyo_core::MediaLocator;
use yoyovideo_desktop::{
    StartupOpen, plan_startup_open,
    DEFAULT_ASPECT, MAX_GRID_TILES, accepted_tile_count, active_after_removal, aspect_from_size,
};

#[test]
fn tiles_are_accepted_up_to_the_cap() {
    // Nothing open yet: a full grid's worth fits.
    assert_eq!(accepted_tile_count(0, MAX_GRID_TILES), (MAX_GRID_TILES, 0));
    assert_eq!(accepted_tile_count(0, 4), (4, 0));
}

#[test]
fn tiles_beyond_the_cap_are_reported_as_dropped() {
    // The caller surfaces the dropped count, rather than silently ignoring files.
    assert_eq!(accepted_tile_count(0, MAX_GRID_TILES + 3), (MAX_GRID_TILES, 3));
    assert_eq!(accepted_tile_count(7, 4), (2, 2));
}

#[test]
fn a_full_grid_accepts_nothing_more() {
    assert_eq!(accepted_tile_count(MAX_GRID_TILES, 2), (0, 2));
}

#[test]
fn removing_a_tile_before_the_active_one_shifts_it_down() {
    // Tile 2 is playing; closing tile 0 must keep tile 2 selected, now at index 1.
    assert_eq!(active_after_removal(Some(2), 0, 4), Some(1));
}

#[test]
fn removing_a_tile_after_the_active_one_leaves_it_alone() {
    assert_eq!(active_after_removal(Some(1), 3, 4), Some(1));
}

#[test]
fn removing_the_active_tile_selects_the_one_that_takes_its_place() {
    assert_eq!(active_after_removal(Some(1), 1, 4), Some(1));
}

#[test]
fn removing_the_last_and_active_tile_falls_back_to_the_new_last() {
    assert_eq!(active_after_removal(Some(3), 3, 4), Some(2));
}

#[test]
fn removing_the_only_tile_clears_the_selection() {
    assert_eq!(active_after_removal(Some(0), 0, 1), None);
}

#[test]
fn removal_without_a_selection_stays_unselected() {
    assert_eq!(active_after_removal(None, 0, 3), None);
}

#[test]
fn aspect_falls_back_until_mpv_reports_a_size() {
    assert_eq!(aspect_from_size(None, None), DEFAULT_ASPECT);
    assert_eq!(aspect_from_size(Some(1920), None), DEFAULT_ASPECT);
    assert_eq!(aspect_from_size(None, Some(1080)), DEFAULT_ASPECT);
}

#[test]
fn aspect_comes_from_the_reported_size() {
    assert!((aspect_from_size(Some(1920), Some(1080)) - 16.0 / 9.0).abs() < 0.001);
    assert!((aspect_from_size(Some(1080), Some(1920)) - 9.0 / 16.0).abs() < 0.001);
    assert!((aspect_from_size(Some(600), Some(600)) - 1.0).abs() < 0.001);
}

#[test]
fn a_zero_dimension_falls_back_instead_of_dividing_by_zero() {
    assert_eq!(aspect_from_size(Some(0), Some(1080)), DEFAULT_ASPECT);
    assert_eq!(aspect_from_size(Some(1920), Some(0)), DEFAULT_ASPECT);
}

#[test]
fn no_arguments_open_nothing() {
    assert_eq!(plan_startup_open(Vec::new()), StartupOpen::Nothing);
}

#[test]
fn paths_that_do_not_exist_are_ignored() {
    // A stray flag or a deleted file must not open a broken tile.
    let plan = plan_startup_open(vec![
        PathBuf::from("Z:/definitely/missing-one.mp4"),
        PathBuf::from("--some-flag"),
    ]);
    assert_eq!(plan, StartupOpen::Nothing);
}

#[test]
fn one_real_file_opens_in_single_video_mode() {
    let file = NamedTempFile::new().unwrap();
    let plan = plan_startup_open(vec![file.path().to_path_buf()]);

    assert_eq!(plan, StartupOpen::Single(MediaLocator::File(file.path().to_path_buf())));
}

#[test]
fn several_real_files_open_as_a_grid() {
    let a = NamedTempFile::new().unwrap();
    let b = NamedTempFile::new().unwrap();
    let plan = plan_startup_open(vec![a.path().to_path_buf(), b.path().to_path_buf()]);

    match plan {
        StartupOpen::Grid(locators) => assert_eq!(locators.len(), 2),
        other => panic!("expected a grid, got {other:?}"),
    }
}

#[test]
fn a_startup_grid_is_capped() {
    let files: Vec<NamedTempFile> =
        (0..MAX_GRID_TILES + 3).map(|_| NamedTempFile::new().unwrap()).collect();
    let plan = plan_startup_open(files.iter().map(|f| f.path().to_path_buf()).collect());

    match plan {
        StartupOpen::Grid(locators) => assert_eq!(locators.len(), MAX_GRID_TILES),
        other => panic!("expected a grid, got {other:?}"),
    }
}
