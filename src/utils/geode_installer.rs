use crate::utils::steam_game_finder::SteamGameFinder;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;

const GD_APP_ID: &str = "322170";
const GEODE_API_URL: &str = "https://api.geode-sdk.org/v1/loader/versions/latest";
const GEODE_GITHUB_URL: &str = "https://github.com/geode-sdk/geode/releases/download";

pub struct GeodeInstaller {
    finder: SteamGameFinder,
    client: Client,
}

#[derive(Debug)]
pub struct InstallationPaths {
    pub game_path: PathBuf,
    pub proton_prefix: PathBuf,
}

impl GeodeInstaller {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            finder: SteamGameFinder::new(),
            client,
        })
    }

    /// Install Geode to Steam's Geometry Dash installation
    pub fn install_to_steam(&self) -> Result<(), String> {
        let steam_root = self.finder.steam_root()
            .ok_or("Can't find Steam installation")?;

        println!("Steam root found at: {:?}", steam_root);

        let paths = self.locate_geometry_dash()?;
        
        println!("Geometry Dash found at: {:?}", paths.game_path);
        println!("Proton prefix found at: {:?}", paths.proton_prefix);

        self.install_to_wine(&paths.proton_prefix, &paths.game_path)?;

        Ok(())
    }

    /// Install Geode to a custom Wine prefix and game directory
    pub fn install_to_wine(&self, prefix: &Path, game_dir: &Path) -> Result<(), String> {
        self.validate_paths(prefix, game_dir)?;

        println!("Installing Geode to: {:?}", game_dir);
        self.install_to_directory(game_dir)?;

        println!("Patching Wine registry...");
        self.patch_wine_registry(prefix)?;

        println!("Geode installation completed!");
        Ok(())
    }

    fn locate_geometry_dash(&self) -> Result<InstallationPaths, String> {
        let game_info = self.finder.get_game_info(GD_APP_ID)
            .ok_or("Can't find Geometry Dash installation")?;

        let proton_prefix = game_info.proton_prefix
            .ok_or("Can't find Proton prefix for Geometry Dash")?;

        Ok(InstallationPaths {
            game_path: game_info.game_path,
            proton_prefix,
        })
    }

    fn validate_paths(&self, prefix: &Path, game_dir: &Path) -> Result<(), String> {
        if !prefix.exists() {
            return Err(format!("Prefix directory doesn't exist: {:?}", prefix));
        }
        if !game_dir.exists() {
            return Err(format!("Game directory doesn't exist: {:?}", game_dir));
        }
        Ok(())
    }

    fn install_to_directory(&self, destination: &Path) -> Result<(), String> {
        let download_url = self.get_download_url()?;
        println!("Downloading Geode...");
        self.download_and_extract(&download_url, destination)
    }

    fn get_download_url(&self) -> Result<String, String> {
        let tag = self.fetch_latest_tag()?;
        Ok(format!("{}/{}/geode-{}-win.zip", GEODE_GITHUB_URL, tag, tag))
    }

    fn fetch_latest_tag(&self) -> Result<String, String> {
        let response = self.http_get(GEODE_API_URL)?;
        let json: Value = serde_json::from_str(&response)
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        if let Some(error) = json["error"].as_str() {
            if !error.is_empty() {
                return Err(format!("Geode API error: {}", error));
            }
        }

        json["payload"]["tag"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "Failed to extract version tag from API response".to_string())
    }

    fn download_and_extract(&self, url: &str, destination: &Path) -> Result<(), String> {
        fs::create_dir_all(destination)
            .map_err(|e| format!("Failed to create destination directory: {}", e))?;

        let zip_path = destination.join("geode_temp.zip");

        self.download_file(url, &zip_path)?;
        self.extract_zip(&zip_path, destination)?;
        
        fs::remove_file(&zip_path)
            .map_err(|e| format!("Failed to remove temporary zip file: {}", e))?;

        Ok(())
    }

    fn http_get(&self, url: &str) -> Result<String, String> {
        let response = self.client
            .get(url)
            .send()
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error {}", response.status()));
        }

        response
            .text()
            .map_err(|e| format!("Failed to read response body: {}", e))
    }

    fn download_file(&self, url: &str, output: &Path) -> Result<(), String> {
        let response = self.client
            .get(url)
            .send()
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );

        let mut file = File::create(output)
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut downloaded = 0u64;
        let mut buffer = vec![0; 8192];

        let mut reader = response;
        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| format!("Failed to read response: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Failed to write file: {}", e))?;

            downloaded += bytes_read as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message("Download complete");

        Ok(())
    }

    fn extract_zip(&self, zip_path: &Path, destination: &Path) -> Result<(), String> {
        let file = File::open(zip_path)
            .map_err(|e| format!("Failed to open zip file: {}", e))?;

        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("Failed to read zip archive: {}", e))?;

        for i in 0..archive.len() {
            self.extract_zip_entry(&mut archive, i, destination)?;
        }

        Ok(())
    }

    fn extract_zip_entry(
        &self,
        archive: &mut ZipArchive<File>,
        index: usize,
        destination: &Path,
    ) -> Result<(), String> {
        let mut file = archive
            .by_index(index)
            .map_err(|e| format!("Failed to access zip entry {}: {}", index, e))?;

        let out_path = match file.enclosed_name() {
            Some(path) => destination.join(path),
            None => return Ok(()), // Skip unsafe paths
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            self.extract_file(&mut file, &out_path)?;
        }

        // Preserve Unix permissions if available
        if let Some(mode) = file.unix_mode() {
            let _ = fs::set_permissions(&out_path, fs::Permissions::from_mode(mode));
        }

        Ok(())
    }

    fn extract_file(&self, zip_file: &mut dyn io::Read, out_path: &Path) -> Result<(), String> {
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }

        let mut out_file = File::create(out_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        io::copy(zip_file, &mut out_file)
            .map_err(|e| format!("Failed to extract file: {}", e))?;

        Ok(())
    }

    fn patch_wine_registry(&self, prefix: &Path) -> Result<(), String> {
        let user_reg = prefix.join("user.reg");
        
        if !user_reg.exists() {
            return Err(format!("Wine registry file not found: {:?}", user_reg));
        }

        let mut content = fs::read_to_string(&user_reg)
            .map_err(|e| format!("Failed to read registry file: {}", e))?;

        self.ensure_dll_override(&mut content);

        fs::write(&user_reg, content)
            .map_err(|e| format!("Failed to write registry file: {}", e))?;

        Ok(())
    }

    fn ensure_dll_override(&self, content: &mut String) {
        const SECTION: &str = "[Software\\\\Wine\\\\DllOverrides]";
        const ENTRY: &str = "\"xinput1_4\"=\"native,builtin\"";

        if content.contains("\"xinput1_4\"=") {
            return; // Already configured
        }

        if !content.contains(SECTION) {
            self.add_dll_overrides_section(content);
        } else {
            self.add_dll_entry_to_section(content, SECTION, ENTRY);
        }
    }

    fn add_dll_overrides_section(&self, content: &mut String) {
        let timestamp = current_timestamp();
        let hex_time = current_hex_timestamp();
        
        content.push_str(&format!(
            "\n\n[Software\\\\Wine\\\\DllOverrides] {}\n#time={}\n\"xinput1_4\"=\"native,builtin\"\n",
            timestamp, hex_time
        ));
    }

    fn add_dll_entry_to_section(&self, content: &mut String, section: &str, entry: &str) {
        if let Some(section_pos) = content.find(section) {
            let search_start = section_pos + section.len();
            
            let insert_pos = content[search_start..]
                .find("\n[")
                .map(|pos| search_start + pos)
                .unwrap_or(content.len());

            let entry_with_newline = if insert_pos == content.len() {
                format!("\n{}\n", entry)
            } else {
                format!("{}\n", entry)
            };

            content.insert_str(insert_pos, &entry_with_newline);
        }
    }
}

impl Default for GeodeInstaller {
    fn default() -> Self {
        Self::new().expect("Failed to initialize GeodeInstaller")
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn current_hex_timestamp() -> String {
    format!("{:x}", current_timestamp())
}