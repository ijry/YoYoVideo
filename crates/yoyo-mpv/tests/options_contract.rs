use yoyo_mpv::{MpvClientOptions, MpvVideoWindow};

#[test]
fn default_options_do_not_force_a_video_window() {
    let options = MpvClientOptions::default();

    assert!(options.video_window.is_none());
    assert!(!options.force_window);
    assert!(options.mpv_option_pairs().is_empty());
}

#[test]
fn video_window_options_are_formatted_for_mpv_before_runtime_init() {
    let options = MpvClientOptions {
        video_window: Some(MpvVideoWindow::new(42)),
        force_window: true,
        profile: Some("low-latency".into()),
    };

    assert_eq!(
        options.mpv_option_pairs(),
        vec![
            ("wid", "42".to_string()),
            ("force-window", "yes".to_string()),
            ("profile", "low-latency".to_string()),
        ]
    );
}
