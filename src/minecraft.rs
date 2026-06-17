use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: isize,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> isize;
}

pub fn minecraft_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Roaming"))
        .join(".minecraft")
}

pub fn mods_dir() -> PathBuf {
    minecraft_dir().join("mods")
}

pub fn bedrock_default_dir() -> PathBuf {
    bedrock_roaming_dir()
}

fn bedrock_roaming_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Roaming"))
        .join("Minecraft Bedrock")
        .join("Users")
        .join("Shared")
        .join("games")
        .join("com.mojang")
}

fn bedrock_package_dir(package: &str) -> Option<PathBuf> {
    Some(
        dirs::data_local_dir()?
            .join("Packages")
            .join(package)
            .join("LocalState")
            .join("games")
            .join("com.mojang"),
    )
}

pub fn find_bedrock_dir() -> Option<PathBuf> {
    let roaming = bedrock_roaming_dir();
    if roaming.exists() {
        return Some(roaming);
    }

    let local = dirs::data_local_dir()?;
    let packages = local.join("Packages");
    let known_packages = [
        "Microsoft.MinecraftUWP_8wekyb3d8bbwe",
        "Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe",
    ];

    for package in known_packages {
        if let Some(dir) = bedrock_package_dir(package) {
            if dir.exists() {
                return Some(dir);
            }
        }
    }

    if let Ok(entries) = fs::read_dir(&packages) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains("minecraft") {
                let dir = entry
                    .path()
                    .join("LocalState")
                    .join("games")
                    .join("com.mojang");
                if dir.exists() {
                    return Some(dir);
                }
            }
        }
    }

    None
}

pub fn bedrock_dir(custom_path: Option<&str>) -> PathBuf {
    if let Some(path) = custom_path {
        if !path.trim().is_empty() {
            return PathBuf::from(path.trim());
        }
    }
    find_bedrock_dir().unwrap_or_else(bedrock_default_dir)
}

pub fn bedrock_behavior_packs_dir(custom_path: Option<&str>) -> PathBuf {
    bedrock_dir(custom_path).join("behavior_packs")
}

pub fn bedrock_resource_packs_dir(custom_path: Option<&str>) -> PathBuf {
    bedrock_dir(custom_path).join("resource_packs")
}

pub fn bedrock_backup_dir(custom_path: Option<&str>) -> PathBuf {
    bedrock_dir(custom_path).join("lightning_packs_backup")
}

pub fn profiles_path() -> PathBuf {
    minecraft_dir().join("launcher_profiles.json")
}

pub fn backup_dir() -> PathBuf {
    minecraft_dir().join("mods_backup")
}

pub fn versions_dir() -> PathBuf {
    minecraft_dir().join("versions")
}

pub fn libraries_dir() -> PathBuf {
    minecraft_dir().join("libraries")
}

pub fn assets_dir() -> PathBuf {
    minecraft_dir().join("assets")
}

fn ensure_mods_dir() -> std::io::Result<()> {
    let dir = mods_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

fn ensure_bedrock_pack_dirs(custom_path: Option<&str>) -> std::io::Result<()> {
    fs::create_dir_all(bedrock_behavior_packs_dir(custom_path))?;
    fs::create_dir_all(bedrock_resource_packs_dir(custom_path))?;
    Ok(())
}

pub fn list_mods() -> std::io::Result<Vec<ModEntry>> {
    ensure_mods_dir()?;
    let dir = mods_dir();
    let mut mods = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "jar") {
            let metadata = fs::metadata(&path)?;
            mods.push(ModEntry {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                path,
                size: metadata.len(),
                modified: metadata.modified().ok(),
            });
        }
    }
    mods.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(mods)
}

pub fn delete_mod(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn delete_entry_path(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn open_mods_folder() -> std::io::Result<()> {
    open_folder(&mods_dir())
}

pub fn open_bedrock_folder(custom_path: Option<&str>) -> std::io::Result<()> {
    let path = bedrock_dir(custom_path);
    fs::create_dir_all(&path)?;
    open_folder(&path)
}

pub fn open_bedrock_behavior_packs_folder(custom_path: Option<&str>) -> std::io::Result<()> {
    let path = bedrock_behavior_packs_dir(custom_path);
    fs::create_dir_all(&path)?;
    open_folder(&path)
}

pub fn open_bedrock_resource_packs_folder(custom_path: Option<&str>) -> std::io::Result<()> {
    let path = bedrock_resource_packs_dir(custom_path);
    fs::create_dir_all(&path)?;
    open_folder(&path)
}

pub fn open_folder(path: &Path) -> std::io::Result<()> {
    let dir = if path.is_dir() {
        path.to_path_buf()
    } else if let Some(parent) = path.parent() {
        parent.to_path_buf()
    } else {
        path.to_path_buf()
    };
    let dir_wide: Vec<u16> = dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let result = unsafe {
        ShellExecuteW(
            0,
            operation.as_ptr(),
            dir_wide.as_ptr(),
            ptr::null(),
            ptr::null(),
            5,
        )
    };
    if result as isize <= 32 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

pub fn find_launcher() -> Option<PathBuf> {
    let common_paths: Vec<&str> = vec![
        r"C:\Program Files (x86)\Minecraft Launcher\MinecraftLauncher.exe",
        r"C:\Program Files (x86)\Minecraft Launcher\minecraft-launcher.exe",
        r"C:\Program Files\Minecraft Launcher\MinecraftLauncher.exe",
        r"C:\Program Files\Minecraft Launcher\minecraft-launcher.exe",
    ];

    for path_str in &common_paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Some(path);
        }
    }

    let windows_apps = PathBuf::from(r"C:\Program Files\WindowsApps");
    if windows_apps.exists() {
        if let Ok(entries) = fs::read_dir(&windows_apps) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                if dir_name.contains("minecraft") {
                    let dir = entry.path();
                    for file in ["MinecraftLauncher.exe", "minecraft-launcher.exe"] {
                        let exe = dir.join(file);
                        if exe.exists() {
                            return Some(exe);
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn launch_minecraft() -> std::io::Result<()> {
    if let Some(launcher) = find_launcher() {
        Command::new(&launcher).spawn()?;
        Ok(())
    } else {
        let uri_wide: Vec<u16> = "minecraft:\0".encode_utf16().collect();
        let operation: Vec<u16> = "open\0".encode_utf16().collect();
        let result = unsafe {
            ShellExecuteW(
                0,
                operation.as_ptr(),
                uri_wide.as_ptr(),
                ptr::null(),
                ptr::null(),
                5,
            )
        };
        if result as isize <= 32 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

pub fn launch_minecraft_with_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("No launcher path set.".to_string());
    }
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(format!("File not found: {path}"));
    }
    Command::new(&p)
        .spawn()
        .map_err(|e| format!("Failed to launch: {e}"))?;
    Ok(())
}

pub fn pick_file_dialog(title: &str, filter_name: &str, extensions: &[&str]) -> Option<String> {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    if !extensions.is_empty() {
        dialog = dialog.add_filter(filter_name, extensions);
    }
    dialog.pick_file().map(|p| p.to_string_lossy().to_string())
}

pub fn pick_folder_dialog(title: &str) -> Option<String> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string())
}

pub fn backup_mods(mods: &[ModEntry]) -> Result<String, String> {
    let base = backup_dir();
    let timestamp = chrono_now_string();
    let backup_path = base.join(&timestamp);
    fs::create_dir_all(&backup_path).map_err(|e| format!("Failed to create backup dir: {e}"))?;

    for mod_entry in mods {
        let dest = backup_path.join(&mod_entry.name);
        fs::copy(&mod_entry.path, &dest)
            .map_err(|e| format!("Failed to back up {}: {e}", mod_entry.name))?;
    }

    Ok(backup_path.to_string_lossy().to_string())
}

pub fn backup_bedrock_packs(
    packs: &[BedrockPackEntry],
    custom_path: Option<&str>,
) -> Result<String, String> {
    let base = bedrock_backup_dir(custom_path);
    let timestamp = chrono_now_string();
    let backup_path = base.join(&timestamp);
    fs::create_dir_all(&backup_path).map_err(|e| format!("Failed to create backup dir: {e}"))?;

    for pack in packs {
        let kind_dir = backup_path.join(pack.kind.folder_name());
        fs::create_dir_all(&kind_dir)
            .map_err(|e| format!("Failed to create backup category: {e}"))?;
        let dest = kind_dir.join(&pack.name);
        copy_path_recursive(&pack.path, &dest)
            .map_err(|e| format!("Failed to back up {}: {e}", pack.name))?;
    }

    Ok(backup_path.to_string_lossy().to_string())
}

fn copy_path_recursive(source: &Path, dest: &Path) -> std::io::Result<()> {
    if source.is_dir() {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_path_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, dest)?;
    }
    Ok(())
}

pub fn clear_backups() -> Result<usize, String> {
    let dir = backup_dir();
    if !dir.exists() {
        return Ok(0);
    }
    let count = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read backup dir: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .count();
    fs::remove_dir_all(&dir).map_err(|e| format!("Failed to clear backups: {e}"))?;
    Ok(count)
}

pub fn clear_bedrock_backups(custom_path: Option<&str>) -> Result<usize, String> {
    let dir = bedrock_backup_dir(custom_path);
    if !dir.exists() {
        return Ok(0);
    }
    let count = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read backup dir: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .count();
    fs::remove_dir_all(&dir).map_err(|e| format!("Failed to clear backups: {e}"))?;
    Ok(count)
}

fn chrono_now_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();

    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    let (year, month, day) = days_to_date(days as i64);

    format!("{year:04}-{month:02}-{day:02}_{hours:02}-{minutes:02}-{seconds:02}")
}

fn days_to_date(mut days: i64) -> (i64, i64, i64) {
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[derive(Clone, Debug)]
pub struct ModEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BedrockPackKind {
    Behavior,
    Resource,
}

impl BedrockPackKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Behavior => "Behavior Pack",
            Self::Resource => "Resource Pack",
        }
    }

    fn folder_name(self) -> &'static str {
        match self {
            Self::Behavior => "behavior_packs",
            Self::Resource => "resource_packs",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BedrockPackEntry {
    pub name: String,
    pub display_name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub kind: BedrockPackKind,
    pub is_folder: bool,
}

impl BedrockPackEntry {
    pub fn selection_id(&self) -> String {
        format!("{}:{}", self.kind.folder_name(), self.name)
    }
}

pub fn list_bedrock_packs(custom_path: Option<&str>) -> std::io::Result<Vec<BedrockPackEntry>> {
    ensure_bedrock_pack_dirs(custom_path)?;
    let mut packs = Vec::new();
    collect_bedrock_packs(
        &bedrock_behavior_packs_dir(custom_path),
        BedrockPackKind::Behavior,
        &mut packs,
    )?;
    collect_bedrock_packs(
        &bedrock_resource_packs_dir(custom_path),
        BedrockPackKind::Resource,
        &mut packs,
    )?;
    packs.sort_by(|a, b| {
        a.kind
            .folder_name()
            .cmp(b.kind.folder_name())
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    Ok(packs)
}

fn collect_bedrock_packs(
    dir: &Path,
    kind: BedrockPackKind,
    packs: &mut Vec<BedrockPackEntry>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::metadata(&path)?;
        let is_folder = metadata.is_dir();
        let is_pack_file = path.extension().map_or(false, |e| {
            let ext = e.to_string_lossy().to_lowercase();
            ext == "mcpack" || ext == "mcaddon" || ext == "zip"
        });

        if !is_folder && !is_pack_file {
            continue;
        }

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let display_name = read_bedrock_manifest_name(&path).unwrap_or_else(|| name.clone());
        let size = if is_folder {
            folder_size(&path).unwrap_or(0)
        } else {
            metadata.len()
        };

        packs.push(BedrockPackEntry {
            name,
            display_name,
            path,
            size,
            modified: metadata.modified().ok(),
            kind,
            is_folder,
        });
    }
    Ok(())
}

fn read_bedrock_manifest_name(path: &Path) -> Option<String> {
    let manifest_path = path.join("manifest.json");
    let content = fs::read_to_string(manifest_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.get("header")
        .and_then(|h| h.get("name"))
        .and_then(|n| n.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

fn folder_size(path: &Path) -> std::io::Result<u64> {
    let mut size = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = fs::metadata(entry.path())?;
        if metadata.is_dir() {
            size += folder_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }
    Ok(size)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LauncherProfile {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub profile_type: serde_json::Value,
    #[serde(default)]
    pub gameDir: Option<String>,
    #[serde(default)]
    pub lastVersionId: Option<String>,
    #[serde(default)]
    pub javaArgs: Option<String>,
    #[serde(default)]
    pub javaDir: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LauncherProfiles {
    pub profiles: std::collections::HashMap<String, LauncherProfile>,
    #[serde(default)]
    pub selectedProfile: Option<String>,
}

pub fn read_profiles() -> Result<LauncherProfiles, String> {
    let path = profiles_path();
    if !path.exists() {
        return Ok(LauncherProfiles {
            profiles: std::collections::HashMap::new(),
            selectedProfile: None,
        });
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read profiles: {e}"))?;
    let mut data: LauncherProfiles =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profiles: {e}"))?;
    data.profiles
        .retain(|key, _| key != "latest-release" && key != "latest-snapshot");
    Ok(data)
}

pub fn save_profiles(profiles: &LauncherProfiles) -> Result<(), String> {
    let path = profiles_path();
    let content = serde_json::to_string_pretty(profiles)
        .map_err(|e| format!("Failed to serialize profiles: {e}"))?;
    fs::write(&path, &content).map_err(|e| format!("Failed to write profiles: {e}"))
}

pub fn open_minecraft_folder() -> std::io::Result<()> {
    let path = minecraft_dir();
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    open_folder(&path)
}

// --- Config persistence ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub custom_launcher_path: String,
    #[serde(default = "default_logging_enabled")]
    pub logging_enabled: bool,
    #[serde(default)]
    pub custom_bedrock_path: String,
}

const fn default_logging_enabled() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            custom_launcher_path: String::new(),
            logging_enabled: true,
            custom_bedrock_path: String::new(),
        }
    }
}

fn config_path() -> PathBuf {
    minecraft_dir().join("mod_manager_config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        return AppConfig::default();
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return AppConfig::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_config(config: &AppConfig) {
    let path = config_path();
    if let Ok(content) = serde_json::to_string_pretty(config) {
        let _ = fs::write(&path, &content);
    }
}

// --- Version manifest and direct profile launching ---

#[derive(Debug, Clone, serde::Deserialize)]
pub struct VersionManifest {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type", default)]
    pub manifest_type: Option<String>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minecraftArguments", default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub arguments: Option<ArgumentsBlock>,
    #[serde(default)]
    pub libraries: Vec<LibraryDef>,
    #[serde(default)]
    pub assets: String,
    #[serde(rename = "assetIndex", default)]
    pub asset_index: Option<AssetIndexDef>,
    #[serde(rename = "inheritsFrom", default)]
    pub inherits_from: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ArgumentsBlock {
    #[serde(default)]
    pub game: Vec<serde_json::Value>,
    #[serde(default)]
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AssetIndexDef {
    pub id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LibraryDef {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloadsDef>,
    #[serde(default)]
    pub natives: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub rules: Option<Vec<RuleDef>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LibraryDownloadsDef {
    #[serde(default)]
    pub artifact: Option<ArtifactDef>,
    #[serde(default)]
    pub classifiers: Option<std::collections::HashMap<String, ArtifactDef>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ArtifactDef {
    pub path: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RuleDef {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsDef>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct OsDef {
    #[serde(default)]
    pub name: Option<String>,
}

fn is_rule_allowed(rules: &[RuleDef]) -> bool {
    let mut allowed = true;
    for rule in rules {
        let os_match = match &rule.os {
            Some(os) => os
                .name
                .as_deref()
                .map_or(true, |name| name.eq_ignore_ascii_case("windows")),
            None => true,
        };
        if os_match {
            match rule.action.as_str() {
                "allow" => allowed = true,
                "disallow" => allowed = false,
                _ => {}
            }
        }
    }
    allowed
}

fn library_to_path(lib: &LibraryDef) -> Option<PathBuf> {
    if let Some(rules) = &lib.rules {
        if !is_rule_allowed(rules) {
            return None;
        }
    }

    if let Some(downloads) = &lib.downloads {
        if let Some(artifact) = &downloads.artifact {
            let p = libraries_dir().join(&artifact.path);
            if p.exists() {
                return Some(p);
            }
        }
        if let Some(classifiers) = &downloads.classifiers {
            if let Some(natives_key) = lib.natives.as_ref().and_then(|n| n.get("windows")) {
                if let Some(native_artifact) = classifiers.get(natives_key) {
                    let p = libraries_dir().join(&native_artifact.path);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    let parts: Vec<&str> = lib.name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = if parts.len() > 3 {
        Some(parts[3])
    } else {
        None
    };

    let filename = match classifier {
        Some(cls) => format!("{artifact}-{version}-{cls}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    let p = libraries_dir()
        .join(&group)
        .join(artifact)
        .join(version)
        .join(&filename);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn resolve_classpath_string(manifest: &VersionManifest) -> String {
    let mut paths: Vec<String> = Vec::new();
    for lib in &manifest.libraries {
        if let Some(p) = library_to_path(lib) {
            paths.push(p.to_string_lossy().to_string());
        }
    }
    if let Some(version_id) = &manifest.id {
        let version_jar = minecraft_dir()
            .join("versions")
            .join(version_id)
            .join(format!("{version_id}.jar"));
        if version_jar.exists() {
            paths.push(version_jar.to_string_lossy().to_string());
        } else if let Some(ref parent_id) = manifest.inherits_from {
            let parent_jar = minecraft_dir()
                .join("versions")
                .join(parent_id)
                .join(format!("{parent_id}.jar"));
            if parent_jar.exists() {
                paths.push(parent_jar.to_string_lossy().to_string());
            }
        }
    }
    paths.join(";")
}

fn find_native_jars(manifest: &VersionManifest) -> Vec<PathBuf> {
    let mut jars = Vec::new();
    for lib in &manifest.libraries {
        if lib.natives.is_some() {
            if let Some(rules) = &lib.rules {
                if !is_rule_allowed(rules) {
                    continue;
                }
            }
            if let Some(downloads) = &lib.downloads {
                if let Some(classifiers) = &downloads.classifiers {
                    if let Some(natives_key) = lib.natives.as_ref().and_then(|n| n.get("windows")) {
                        if let Some(artifact) = classifiers.get(natives_key) {
                            let p = libraries_dir().join(&artifact.path);
                            if p.exists() {
                                jars.push(p);
                            }
                        }
                    }
                }
            }
        }
    }
    jars
}

fn extract_natives(native_jars: &[PathBuf], target_dir: &Path) -> Result<(), String> {
    if native_jars.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(target_dir).map_err(|e| format!("Failed to create natives dir: {e}"))?;

    for jar_path in native_jars {
        let file = fs::File::open(jar_path)
            .map_err(|e| format!("Failed to open {}: {e}", jar_path.display()))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to read zip {}: {e}", jar_path.display()))?;

        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let name = entry.name().to_string();
            let is_dir = entry.is_dir();

            if !is_dir
                && (name.ends_with(".dll") || name.ends_with(".so") || name.ends_with(".dylib"))
            {
                let out_path = target_dir.join(&name);
                if let Some(parent) = out_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(mut out) = fs::File::create(&out_path) {
                    let _ = std::io::copy(&mut entry, &mut out);
                }
            }
        }
    }
    Ok(())
}

fn replace_placeholders(
    input: &str,
    username: &str,
    token: &str,
    uuid: &str,
    game_dir: &str,
    assets_root: &str,
    asset_index: &str,
    version_name: &str,
    version_type: &str,
) -> String {
    input
        .replace("${auth_player_name}", username)
        .replace("${auth_access_token}", token)
        .replace("${auth_session}", token)
        .replace("${auth_uuid}", uuid)
        .replace("${user_properties}", "{}")
        .replace("${user_type}", "mojang")
        .replace("${game_directory}", game_dir)
        .replace("${game_assets}", assets_root)
        .replace("${assets_root}", assets_root)
        .replace("${assets_index_name}", asset_index)
        .replace("${version_name}", version_name)
        .replace("${version_type}", version_type)
        .replace("${natives_directory}", "{natives_dir}")
        .replace("${classpath}", "{classpath}")
        .replace("${launcher_name}", "lightning-manager")
        .replace("${launcher_version}", "0.1.0")
}

fn resolve_placeholder_arg(
    val: &serde_json::Value,
    username: &str,
    token: &str,
    uuid: &str,
    game_dir: &str,
    assets_root: &str,
    asset_index: &str,
    version_name: &str,
    version_type: &str,
) -> Option<String> {
    match val {
        serde_json::Value::String(s) => Some(replace_placeholders(
            s,
            username,
            token,
            uuid,
            game_dir,
            assets_root,
            asset_index,
            version_name,
            version_type,
        )),
        serde_json::Value::Object(obj) => {
            let mut result = None;
            if let Some(rules) = obj.get("rules").and_then(|r| r.as_array()) {
                let rules_parsed: Vec<RuleDef> = rules
                    .iter()
                    .filter_map(|r| serde_json::from_value(r.clone()).ok())
                    .collect();
                if !rules_parsed.is_empty() && !is_rule_allowed(&rules_parsed) {
                    return None;
                }
            }
            if let Some(value) = obj.get("value") {
                match value {
                    serde_json::Value::String(s) => {
                        result = Some(replace_placeholders(
                            s,
                            username,
                            token,
                            uuid,
                            game_dir,
                            assets_root,
                            asset_index,
                            version_name,
                            version_type,
                        ));
                    }
                    serde_json::Value::Array(arr) => {
                        let mut parts = Vec::new();
                        for v in arr {
                            if let serde_json::Value::String(s) = v {
                                parts.push(replace_placeholders(
                                    s,
                                    username,
                                    token,
                                    uuid,
                                    game_dir,
                                    assets_root,
                                    asset_index,
                                    version_name,
                                    version_type,
                                ));
                            }
                        }
                        if !parts.is_empty() {
                            result = Some(parts.join(" "));
                        }
                    }
                    _ => {}
                }
            }
            result
        }
        _ => None,
    }
}

pub fn read_version_manifest(version_id: &str) -> Result<VersionManifest, String> {
    let path = versions_dir()
        .join(version_id)
        .join(format!("{version_id}.json"));
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read version manifest: {e}"))?;
    let mut manifest: VersionManifest = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse version manifest: {e}"))?;

    if let Some(ref parent_id) = manifest.inherits_from.clone() {
        let parent = read_version_manifest(parent_id)?;
        let mut merged_libs = parent.libraries;
        merged_libs.extend(manifest.libraries);
        manifest.libraries = merged_libs;

        if manifest.assets.is_empty() {
            manifest.assets = parent.assets;
        }
        if manifest.asset_index.is_none() {
            manifest.asset_index = parent.asset_index;
        }

        if manifest.arguments.is_some() && manifest.minecraft_arguments.is_none() {
            if let Some(parent_mc) = &parent.minecraft_arguments {
                let parent_entries: Vec<serde_json::Value> = shell_words_split(parent_mc)
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect();
                let mut combined = parent_entries;
                if let Some(game_args) = &manifest.arguments {
                    combined.extend(game_args.game.clone());
                }
                manifest.arguments = Some(ArgumentsBlock {
                    game: combined,
                    jvm: Vec::new(),
                });
            }
        }
        if manifest.minecraft_arguments.is_none() && manifest.arguments.is_none() {
            manifest.minecraft_arguments = parent.minecraft_arguments;
            manifest.arguments = parent.arguments;
        }
    }

    Ok(manifest)
}

pub fn read_auth_data() -> Result<(String, String, String), String> {
    let path = profiles_path();
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read auth data: {e}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse auth data: {e}"))?;

    if let Some(selected_user) = json.get("selectedUser") {
        let account_uuid = selected_user
            .get("account")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if let Some(auth_db) = json.get("authenticationDatabase") {
            if let Some(account) = auth_db.get(account_uuid) {
                let username = account
                    .get("displayName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Player");
                let access_token = account
                    .get("accessToken")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let uuid = account.get("uuid").and_then(|v| v.as_str()).unwrap_or("0");
                return Ok((
                    username.to_string(),
                    access_token.to_string(),
                    uuid.to_string(),
                ));
            }
        }
    }

    Ok(("Player".to_string(), "0".to_string(), "0".to_string()))
}

pub fn find_java(java_dir: &Option<String>) -> Result<PathBuf, String> {
    if let Some(dir) = java_dir {
        let p = PathBuf::from(dir);
        if p.is_file() {
            return Ok(p);
        }
        let exe = p.join("bin").join("javaw.exe");
        if exe.exists() {
            return Ok(exe);
        }
        let exe = p.join("bin").join("java.exe");
        if exe.exists() {
            return Ok(exe);
        }
        let exe = p.join("javaw.exe");
        if exe.exists() {
            return Ok(exe);
        }
        let exe = p.join("java.exe");
        if exe.exists() {
            return Ok(exe);
        }
    }

    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let home = PathBuf::from(&java_home);
        let exe = home.join("bin").join("javaw.exe");
        if exe.exists() {
            return Ok(exe);
        }
        let exe = home.join("bin").join("java.exe");
        if exe.exists() {
            return Ok(exe);
        }
    }

    if let Ok(paths) = std::env::var("PATH") {
        for dir in paths.split(';') {
            let exe = PathBuf::from(dir).join("javaw.exe");
            if exe.exists() {
                return Ok(exe);
            }
            let exe = PathBuf::from(dir).join("java.exe");
            if exe.exists() {
                return Ok(exe);
            }
        }
    }

    Err("Could not find Java. Set a Java path in the profile or install the JDK/JRE.".to_string())
}

fn ensure_windowed_mode(game_dir: &str) {
    let options_path = std::path::Path::new(game_dir).join("options.txt");
    if let Ok(content) = fs::read_to_string(&options_path) {
        if content.contains("fullscreen:true") {
            let patched = content.replace("fullscreen:true", "fullscreen:false");
            let _ = fs::write(&options_path, &patched);
        }
    }
}

pub fn launch_profile_direct(
    profile: &LauncherProfile,
    manifest: &VersionManifest,
    logging_enabled: bool,
) -> Result<(), String> {
    let (username, access_token, uuid) = read_auth_data()?;

    let classpath = resolve_classpath_string(manifest);
    let java_exe = find_java(&profile.javaDir)?;

    let game_dir = profile
        .gameDir
        .clone()
        .unwrap_or_else(|| minecraft_dir().to_string_lossy().to_string());
    ensure_windowed_mode(&game_dir);
    let assets_root = assets_dir().to_string_lossy().to_string();
    let asset_index = manifest
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .unwrap_or_else(|| manifest.assets.clone());
    let version_name = manifest.id.as_deref().unwrap_or("unknown");
    let version_type = manifest.manifest_type.as_deref().unwrap_or("release");

    let natives_dir = std::env::temp_dir().join(format!(
        "lightning_natives_{}",
        uuid.replace('-', "").chars().take(8).collect::<String>()
    ));
    let native_jars = find_native_jars(manifest);
    extract_natives(&native_jars, &natives_dir)?;

    let mut jvm_args: Vec<String> = Vec::new();

    if let Some(java_args) = &profile.javaArgs {
        let parsed = shell_words_split(java_args);
        let custom: Vec<&str> = parsed
            .iter()
            .map(|s| s.as_str())
            .filter(|s| !s.starts_with("-Djava.library.path"))
            .collect();
        if custom.is_empty() {
            jvm_args.push("-Xmx2G".to_string());
        } else {
            for arg in custom {
                jvm_args.push(arg.to_string());
            }
        }
    } else {
        jvm_args.push("-Xmx2G".to_string());
    }

    jvm_args.push(format!(
        r"-Djava.library.path={}",
        natives_dir.to_string_lossy()
    ));
    jvm_args.push("-cp".to_string());
    let lib_count = classpath.matches(".jar").count();
    jvm_args.push(classpath);

    jvm_args.push(manifest.main_class.clone());

    if let Some(args_block) = &manifest.arguments {
        for val in &args_block.game {
            if let Some(arg) = resolve_placeholder_arg(
                val,
                &username,
                &access_token,
                &uuid,
                &game_dir,
                &assets_root,
                &asset_index,
                version_name,
                version_type,
            ) {
                for a in shell_words_split(&arg) {
                    jvm_args.push(a);
                }
            }
        }
    } else if let Some(mc_args) = &manifest.minecraft_arguments {
        let parsed = replace_placeholders(
            mc_args,
            &username,
            &access_token,
            &uuid,
            &game_dir,
            &assets_root,
            &asset_index,
            version_name,
            version_type,
        );
        for arg in shell_words_split(&parsed) {
            jvm_args.push(arg);
        }
    }

    if logging_enabled {
        let log_path = minecraft_dir().join("lightning_launch.log");
        let cmd_line = format!("\"{}\" {}", java_exe.display(), jvm_args.join(" "));
        let log_entry = format!(
            "--- Minecraft Direct Launch ---\nJava: {}\nLibraries: {}\nCommand:\n{}\n\n",
            java_exe.display(),
            lib_count,
            cmd_line,
        );
        let _ = fs::write(&log_path, &log_entry);

        let log_out = fs::OpenOptions::new()
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Failed to open log for output: {e}"))?;
        let log_err = log_out
            .try_clone()
            .map_err(|e| format!("Failed to clone log handle: {e}"))?;

        let mut child = Command::new(&java_exe)
            .args(&jvm_args)
            .stdout(log_out)
            .stderr(log_err)
            .spawn()
            .map_err(|e| format!("Failed to launch Minecraft: {e}"))?;

        std::thread::sleep(std::time::Duration::from_millis(1000));

        match child.try_wait() {
            Ok(Some(status)) => {
                let mut log = fs::read_to_string(&log_path).unwrap_or(log_entry);
                log.push_str(&format!("\n--- Process exited with code {status} ---\n"));
                let _ = fs::write(&log_path, &log);
                return Err(format!(
                    "Minecraft exited immediately (code {status}). Check the launch log:\n{}",
                    log_path.display()
                ));
            }
            Ok(None) => {}
            Err(e) => {
                return Err(format!("Error checking Java process: {e}"));
            }
        }
    } else {
        let mut child = Command::new(&java_exe)
            .args(&jvm_args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Minecraft: {e}"))?;

        std::thread::sleep(std::time::Duration::from_millis(1000));

        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!("Minecraft exited immediately (code {status})."));
            }
            Ok(None) => {}
            Err(e) => {
                return Err(format!("Error checking Java process: {e}"));
            }
        }
    }

    Ok(())
}

fn shell_words_split(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' {
            if i + 1 < bytes.len() && (bytes[i + 1] == b'"' || bytes[i + 1] == b'\\') {
                i += 1;
                current.push(bytes[i] as char);
                i += 1;
                continue;
            }
            current.push('\\');
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            in_quote = !in_quote;
            i += 1;
            continue;
        }
        if (bytes[i] as char).is_whitespace() && !in_quote {
            if !current.is_empty() {
                result.push(current.clone());
                current.clear();
            }
            i += 1;
            continue;
        }
        current.push(bytes[i] as char);
        i += 1;
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}
