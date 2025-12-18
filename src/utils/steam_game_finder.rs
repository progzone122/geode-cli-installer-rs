use homedir::my_home;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct GameInfo {
    pub app_id: String,
    pub game_path: PathBuf,
    pub proton_prefix: Option<PathBuf>,
    pub library_path: PathBuf,
}

pub struct SteamGameFinder {
    steam_root: Option<PathBuf>,
    library_folders: Vec<PathBuf>,
}

impl SteamGameFinder {
    pub fn new() -> Self {
        let steam_root = Self::find_steam_root();
        let library_folders = Self::discover_library_folders(&steam_root);
        
        Self {
            steam_root,
            library_folders,
        }
    }

    pub fn steam_root(&self) -> Option<&PathBuf> {
        self.steam_root.as_ref()
    }


    #[allow(unused)]
    pub fn library_folders(&self) -> &[PathBuf] {
        &self.library_folders
    }

    pub fn get_game_info(&self, app_id: &str) -> Option<GameInfo> {
        let (game_path, library_path) = self.find_game_by_appid(app_id)?;
        let proton_prefix = self.find_proton_prefix(app_id, Some(&library_path));

        Some(GameInfo {
            app_id: app_id.to_string(),
            game_path,
            library_path,
            proton_prefix,
        })
    }

    fn find_steam_root() -> Option<PathBuf> {
        let home = my_home().ok()??;

        let candidates = [
            home.join(".steam/steam"),
            home.join(".steam/root"),
            home.join(".local/share/Steam"),
            home.join(".var/app/com.valvesoftware.Steam"),
            home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
            PathBuf::from("/usr/share/steam"),
        ];

        candidates.into_iter()
            .find(|path| path.exists() && path.join("steamapps").exists())
    }

    fn discover_library_folders(steam_root: &Option<PathBuf>) -> Vec<PathBuf> {
        let steam_root = match steam_root {
            Some(root) => root,
            None => return Vec::new(),
        };

        let mut folders = vec![steam_root.join("steamapps")];
        folders.extend(Self::parse_library_folders_vdf(steam_root));
        Self::deduplicate_paths(folders)
    }

    fn parse_library_folders_vdf(steam_root: &PathBuf) -> Vec<PathBuf> {
        let library_file = steam_root.join("steamapps/libraryfolders.vdf");
        if !library_file.exists() {
            return Vec::new();
        }

        let data = VdfParser::parse_file(&library_file);
        
        data.iter()
            .filter(|(key, _)| key.contains(".path"))
            .filter_map(|(_, value)| {
                let path = PathBuf::from(value).join("steamapps");
                path.exists().then_some(path)
            })
            .collect()
    }

    fn deduplicate_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut seen = HashSet::new();
        paths.into_iter()
            .filter(|path| seen.insert(path.to_string_lossy().to_string()))
            .collect()
    }

    fn find_game_by_appid(&self, app_id: &str) -> Option<(PathBuf, PathBuf)> {
        for library_path in &self.library_folders {
            if let Some(game_info) = self.check_library_for_game(library_path, app_id) {
                return Some(game_info);
            }
        }
        None
    }

    fn check_library_for_game(&self, library_path: &PathBuf, app_id: &str) -> Option<(PathBuf, PathBuf)> {
        let acf_file = library_path.join(format!("appmanifest_{}.acf", app_id));
        
        if !acf_file.exists() {
            return None;
        }

        let acf_data = VdfParser::parse_file(&acf_file);
        let install_dir = acf_data.get("AppState.installdir")?;
        let game_path = library_path.join("common").join(install_dir);
        
        game_path.exists().then_some((game_path, library_path.clone()))
    }

    fn find_proton_prefix(&self, app_id: &str, preferred_library: Option<&PathBuf>) -> Option<PathBuf> {
        // Check preferred library first
        if let Some(prefix) = preferred_library.and_then(|lib| Self::check_compatdata(lib, app_id)) {
            return Some(prefix);
        }

        // Fall back to searching all libraries
        self.library_folders.iter()
            .find_map(|lib| Self::check_compatdata(lib, app_id))
    }

    fn check_compatdata(library_path: &PathBuf, app_id: &str) -> Option<PathBuf> {
        let compatdata_path = library_path
            .join("compatdata")
            .join(app_id)
            .join("pfx");
        
        compatdata_path.exists().then_some(compatdata_path)
    }
}

impl Default for SteamGameFinder {
    fn default() -> Self {
        Self::new()
    }
}

/// VDF (Valve Data Format) parser
struct VdfParser;

impl VdfParser {
    fn parse_file(path: &PathBuf) -> HashMap<String, String> {
        if !path.exists() {
            return HashMap::new();
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };

        let mut result = HashMap::new();
        let mut pos = 0;
        Self::parse_recursive(&content, &mut pos, &mut result, String::new());
        result
    }

    fn parse_recursive(
        content: &str,
        pos: &mut usize,
        result: &mut HashMap<String, String>,
        prefix: String,
    ) {
        let chars: Vec<char> = content.chars().collect();

        while *pos < chars.len() {
            Self::skip_whitespace(&chars, pos);
            
            if *pos >= chars.len() {
                break;
            }

            if Self::skip_comment(&chars, pos) {
                continue;
            }

            if Self::handle_closing_brace(&chars, pos) {
                return;
            }

            if Self::handle_opening_brace(&chars, pos) {
                continue;
            }

            if chars[*pos] == '"' {
                Self::parse_key_value(&chars, pos, result, &prefix, content);
            } else {
                *pos += 1;
            }
        }
    }

    fn skip_whitespace(chars: &[char], pos: &mut usize) {
        while *pos < chars.len() && chars[*pos].is_whitespace() {
            *pos += 1;
        }
    }

    fn skip_comment(chars: &[char], pos: &mut usize) -> bool {
        if *pos + 1 < chars.len() && chars[*pos] == '/' && chars[*pos + 1] == '/' {
            while *pos < chars.len() && chars[*pos] != '\n' {
                *pos += 1;
            }
            return true;
        }
        false
    }

    fn handle_closing_brace(chars: &[char], pos: &mut usize) -> bool {
        if chars[*pos] == '}' {
            *pos += 1;
            return true;
        }
        false
    }

    fn handle_opening_brace(chars: &[char], pos: &mut usize) -> bool {
        if chars[*pos] == '{' {
            *pos += 1;
            return true;
        }
        false
    }

    fn parse_key_value(
        chars: &[char],
        pos: &mut usize,
        result: &mut HashMap<String, String>,
        prefix: &str,
        content: &str,
    ) {
        *pos += 1; // Skip opening quote
        
        let key = Self::read_quoted_string(chars, pos);
        Self::skip_whitespace(chars, pos);

        if *pos < chars.len() && chars[*pos] == '"' {
            *pos += 1;
            let value = Self::read_quoted_string(chars, pos);
            let full_key = Self::build_key(prefix, &key);
            result.insert(full_key, value);
        } else if *pos < chars.len() && chars[*pos] == '{' {
            *pos += 1;
            let new_prefix = Self::build_key(prefix, &key);
            Self::parse_recursive(content, pos, result, new_prefix);
        }
    }

    fn read_quoted_string(chars: &[char], pos: &mut usize) -> String {
        let mut s = String::new();
        while *pos < chars.len() && chars[*pos] != '"' {
            s.push(chars[*pos]);
            *pos += 1;
        }
        *pos += 1; // Skip closing quote
        s
    }

    fn build_key(prefix: &str, key: &str) -> String {
        if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", prefix, key)
        }
    }
}