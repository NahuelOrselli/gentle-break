fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "get_state",
                "save_state",
                "toggle_panel",
                "hide_panel",
                "quit",
                "import_sound",
                "preview_sound",
                "remove_sound",
                "peek_shout_layout",
                "begin_shout",
                "end_shout",
                "preview_reaction",
            ]),
        ),
    )
    .expect("failed to run tauri-build");
}
