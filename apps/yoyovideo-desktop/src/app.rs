slint::include_modules!();

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_target(false).init();
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()?;

    let app = MainWindow::new()?;
    app.run()?;
    Ok(())
}
