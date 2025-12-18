struct GameInfo {
    app_id: String,
    game_path: Option<std::path::PathBuf>,
    proton_prefix: Option<std::path::PathBuf>,
    library_path: Option<std::path::PathBuf>,
    found: bool
}