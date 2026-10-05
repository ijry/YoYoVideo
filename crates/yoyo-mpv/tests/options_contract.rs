use yoyo_mpv::{MpvClientOptions, MpvVideoWindow};

#[test]
fn default_options_do_not_force_a_video_window() {
    let options = MpvClientOptions::default();

    assert!(options.video_window.is_none());
    assert!(!options.force_window);
    assert!(options.audio_output.is_none());
    assert!(options.mpv_option_pairs().is_empty());
}

#[test]
fn video_window_options_are_formatted_for_mpv_before_runtime_init() {
    let options = MpvClientOptions {
        video_window: Some(MpvVideoWindow::new(42)),
        force_window: true,
        profile: Some("low-latency".into()),
        ..MpvClientOptions::default()
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

/// libmpv is a library and never reads `mpv.conf`, so an audio output that is not
/// mpv's own default can only be selected by passing the option in. That is what
/// keeps the release smoke test working on a CI runner with no sound card.
#[test]
fn audio_output_is_passed_as_the_ao_option() {
    let options =
        MpvClientOptions { audio_output: Some("null".into()), ..MpvClientOptions::default() };

    assert_eq!(options.mpv_option_pairs(), vec![("ao", "null".to_string())]);
}
