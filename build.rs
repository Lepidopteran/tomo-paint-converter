use std::env::var;

fn main() {
    if var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        embed_resource::compile("resources/res.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }

    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
}
