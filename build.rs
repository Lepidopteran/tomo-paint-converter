fn main() {
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
    #[cfg(windows)]
    {
        embed_resource::compile("assets/icon.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
