#[cfg(test)]
mod tests {
    use geode_cli_installer::utils::steam_game_finder::SteamGameFinder;

    #[test]
    fn test_steam_finder() {
        let finder = SteamGameFinder::new();
        println!("Steam root: {:?}", finder.steam_root());
        println!("Library folders: {:?}", finder.library_folders());
    }
}