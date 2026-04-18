use color_eyre::eyre::Result;

slint::include_modules!();

pub fn run() -> Result<()> {
    let app = AppWindow::new()?;
    app.run()?;

    Ok(())
}
