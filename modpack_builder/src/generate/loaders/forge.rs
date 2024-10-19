use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt::{Display, Debug},
    io::Write as _,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use shared::{
    files::{self, get_files_in_dir},
    java::{download_java, get_java},
    paths::{get_java_dir, get_libraries_dir, get_versions_dir},
    progress::ProgressBar as _,
    version::version_metadata::{
        fetch_version_metadata, get_version_metadata_path, read_version_metadata,
        save_version_metadata,
    },
};

use crate::{
    generate::{
        loaders::vanilla::VanillaGenerator, patch::replace_download_urls, sync::sync_version,
    },
    progress::TerminalProgressBar,
    utils::{exec_custom_command_in_dir, get_vanilla_version_info, to_abs_path_str},
};

use super::generator::{GeneratorResult, VersionGenerator};

const FORGE_MAVEN_METADATA_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";

const FORGE_PROMOTIONS_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

const NEOFORGE_MAVEN_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

#[derive(Debug, Deserialize)]
struct ForgeMavenMetadata {
    versions: HashMap<String, Vec<String>>,
}

impl ForgeMavenMetadata {
    async fn from_url(url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let response = client.get(url).send().await?.error_for_status()?;
        Ok(ForgeMavenMetadata {
            versions: response.json().await?,
        })
    }

    fn has_version(&self, minecraft_version: &str, forge_version: &str) -> bool {
        self.versions
            .get(minecraft_version)
            .map_or(false, |versions| {
                versions.contains(&format!("{}-{}", minecraft_version, forge_version))
            })
    }
}

#[derive(Debug, Deserialize)]
struct NeoforgeMavenMetadata {
    versioning: Versioning,
}

#[derive(Debug, Deserialize)]
struct Versioning {
    versions: Versions,
}

#[derive(Debug, Deserialize)]
struct Versions {
    version: Vec<String>,
}

impl NeoforgeMavenMetadata {
    async fn from_url(url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let response = client.get(url).send().await?.error_for_status()?;
        let metadata: NeoforgeMavenMetadata = serde_xml_rs::from_str(&response.text().await?)?;
        Ok(metadata)
    }

    fn get_latest_matching_version(&self, minecraft_version: &str) -> Option<String> {
        let mut mc_version_parts: Vec<&str> = minecraft_version.split('.').collect();
        if mc_version_parts.len() < 2 {
            return None;
        }
        if mc_version_parts.len() == 2 {
            mc_version_parts.push("0");
        }

        let mc_version_prefix = format!("{}.{}", mc_version_parts[1], mc_version_parts[2]);
        self.versioning
            .versions
            .version
            .iter()
            .filter(|&version| version.starts_with(&mc_version_prefix))
            .max_by(|a, b| {
                let a_parts: Vec<u32> = a.split(|c: char| !c.is_digit(10)).filter_map(|s| s.parse().ok()).collect();
                let b_parts: Vec<u32> = b.split(|c: char| !c.is_digit(10)).filter_map(|s| s.parse().ok()).collect();
                a_parts.cmp(&b_parts)
            })
            .cloned()
    }

    fn has_version(&self, version: &str) -> bool {
        self.versioning
            .versions
            .version
            .contains(&version.to_string())
    }
}

#[derive(Deserialize)]
struct ForgePromotions {
    promos: HashMap<String, String>,
}

impl ForgePromotions {
    async fn from_url(url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let response = client.get(url).send().await?.error_for_status()?;
        let promotions: ForgePromotions = response.json().await?;
        Ok(promotions)
    }

    fn get_latest_version(&self, minecraft_version: &str, version_type: &str) -> Option<String> {
        self.promos
            .get(&format!("{}-{}", minecraft_version, version_type))
            .cloned()
    }
}

const FORGE_INSTALLER_BASE_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/";

const NEOFORGE_INSTALLER_BASE_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/";

async fn download_forge_installer(
    full_version: &str,
    work_dir: &Path,
    loader: &Loader,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let filename = format!("{:?}-{}-installer.jar", loader, full_version);
    let forge_installer_url = match loader {
        Loader::Forge => format!("{}{}/{}", FORGE_INSTALLER_BASE_URL, full_version, filename),
        Loader::Neoforge => format!(
            "{}{}/{}",
            NEOFORGE_INSTALLER_BASE_URL, full_version, filename
        ),
    };
    let forge_installer_path = work_dir.join(filename);
    let client = Client::new();
    files::download_file(&client, &forge_installer_url, &forge_installer_path).await?;
    Ok(forge_installer_path)
}

#[derive(Deserialize)]
struct ProfileInfo {
    #[serde(rename = "lastVersionId")]
    last_version_id: String,
}

#[derive(Deserialize)]
pub struct LauncherProfiles {
    profiles: HashMap<String, ProfileInfo>,
}

pub enum Loader {
    Forge,
    Neoforge,
}

impl Display for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loader::Forge => write!(f, "Forge"),
            Loader::Neoforge => write!(f, "Neoforge"),
        }
    }
}

impl Debug for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loader::Forge => write!(f, "forge"),
            Loader::Neoforge => write!(f, "neoforge"),
        }
    }
}

pub struct ForgeGenerator {
    loader: Loader,
    version_name: String,
    minecraft_version: String,
    loader_version: Option<String>,
    download_server_base: String,
    replace_download_urls: bool,
}

impl ForgeGenerator {
    pub fn new(
        loader: Loader,
        version_name: String,
        minecraft_version: String,
        loader_version: Option<String>,
        download_server_base: String,
        replace_download_urls: bool,
    ) -> Self {
        Self {
            loader,
            version_name,
            minecraft_version,
            loader_version,
            download_server_base,
            replace_download_urls,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ForgeError {
    #[error("Forge version {0} not found for minecraft {1}")]
    ForgeVersionNotFound(String, String),
    #[error("No forge profiles found")]
    NoForgeProfiles,
}

pub async fn get_forge_version(
    minecraft_version: &str,
    loader_version: &Option<String>,
    loader: &Loader,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    match loader {
        Loader::Forge => {
            let forge_promotions = ForgePromotions::from_url(FORGE_PROMOTIONS_URL).await?;

            let forge_version = match loader_version {
                Some(version) => version.to_string(),
                None => {
                    const FORGE_DEFAULT: &str = "recommended";
                    info!("Version not set, using \"{}\"", FORGE_DEFAULT);
                    forge_promotions
                        .get_latest_version(minecraft_version, FORGE_DEFAULT)
                        .ok_or_else(|| {
                            ForgeError::ForgeVersionNotFound(
                                FORGE_DEFAULT.to_string(),
                                minecraft_version.to_string(),
                            )
                        })?
                }
            };

            let forge_maven_metadata =
                ForgeMavenMetadata::from_url(FORGE_MAVEN_METADATA_URL).await?;
            if forge_maven_metadata.has_version(minecraft_version, &forge_version) {
                return Ok(forge_version);
            }
        }
        Loader::Neoforge => {
            let neoforge_maven_metadata =
                NeoforgeMavenMetadata::from_url(NEOFORGE_MAVEN_METADATA_URL).await?;

            let neoforge_version = match loader_version {
                Some(version) => version.to_string(),
                None => {
                    info!("Version not set, using latest");
                    neoforge_maven_metadata
                        .get_latest_matching_version(minecraft_version)
                        .ok_or_else(|| {
                            ForgeError::ForgeVersionNotFound(
                                "neoforge:latest".to_string(),
                                minecraft_version.to_string(),
                            )
                        })?
                }
            };

            if neoforge_maven_metadata.has_version(&neoforge_version) {
                return Ok(neoforge_version);
            }
        }
    };

    let forge_version = loader_version.as_deref().unwrap_or("default");
    error!(
        "{} version {} not found for minecraft {}",
        loader, forge_version, minecraft_version
    );
    Err(Box::new(ForgeError::ForgeVersionNotFound(
        forge_version.to_string(),
        minecraft_version.to_string(),
    )))
}

pub async fn get_vanilla_java_version(
    minecraft_version: &str,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let version_info = get_vanilla_version_info(minecraft_version).await?;
    let version_metadata = fetch_version_metadata(&version_info).await?;
    Ok(version_metadata
        .java_version
        .map(|v| v.major_version.to_string()))
}

// trick forge installer into thinking that the folder is actually a minecraft instance
pub fn trick_forge(
    forge_work_dir: &Path,
    minecraft_version: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    std::fs::create_dir_all(forge_work_dir.join("versions").join(minecraft_version))?;
    let mut file = std::fs::File::create(forge_work_dir.join("launcher_profiles.json"))?;
    file.write(b"{}")?;
    Ok(())
}

pub fn get_full_version(minecraft_version: &str, forge_version: &str) -> String {
    format!("{}-{}", minecraft_version, forge_version)
}

pub async fn install_forge(
    forge_work_dir: &Path,
    java_dir: &Path,
    forge_version: &str,
    minecraft_version: &str,
    loader: &Loader,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    std::fs::create_dir_all(forge_work_dir)?;

    let lock_file = forge_work_dir.join("forge.lock");

    if !lock_file.exists() {
        let java_version = get_vanilla_java_version(minecraft_version)
            .await?
            .map_or_else(
                || {
                    warn!("Java version not found, using default");
                    "8".to_string()
                },
                |v| v,
            );

        info!("Getting java {}", &java_version);
        let java_installation;
        if let Some(existing_java_installation) = get_java(&java_version, &java_dir) {
            java_installation = existing_java_installation;
        } else {
            info!("Java installation not found, downloading");

            let progress_bar = Arc::new(TerminalProgressBar::new());

            progress_bar.set_message("Downloading java...");
            java_installation = download_java(&java_version, &java_dir, progress_bar).await?;
        }

        info!("Downloading forge installer");
        let full_version = match loader {
            Loader::Forge => get_full_version(minecraft_version, forge_version),
            Loader::Neoforge => forge_version.to_string(),
        };
        let forge_installer_path =
            download_forge_installer(&full_version, forge_work_dir, loader).await?;

        trick_forge(forge_work_dir, minecraft_version)?;

        info!("Running forge installer");
        let install_client_flag = match loader {
            Loader::Forge => "--installClient",
            Loader::Neoforge => "--install-client",
        };
        exec_custom_command_in_dir(
            &format!(
                "{} -jar {} {} .",
                to_abs_path_str(&java_installation.path)?,
                to_abs_path_str(&forge_installer_path)?,
                install_client_flag,
            ),
            &forge_work_dir,
        )
        .await?;

        std::fs::File::create(lock_file)?;
    } else {
        info!(
            "Forge {} already present, skipping installation",
            forge_version
        );
    }

    let launcher_profiles_path = forge_work_dir.join("launcher_profiles.json");
    let launcher_profiles_content = std::fs::read_to_string(&launcher_profiles_path)?;
    let launcher_profiles: LauncherProfiles = serde_json::from_str(&launcher_profiles_content)?;

    let id = launcher_profiles
        .profiles
        .values()
        .next()
        .ok_or(ForgeError::NoForgeProfiles)?
        .last_version_id
        .clone();

    Ok(id)
}

#[async_trait]
impl VersionGenerator for ForgeGenerator {
    async fn generate(
        &self,
        output_dir: &Path,
        work_dir: &Path,
    ) -> Result<GeneratorResult, Box<dyn Error + Send + Sync>> {
        info!(
            "Generating {} modpack \"{}\", minecraft version {}",
            self.loader, self.version_name, self.minecraft_version
        );

        info!("Generating vanilla version first");
        let vanilla_generator = VanillaGenerator::new(
            self.version_name.clone(),
            self.minecraft_version.clone(),
            self.download_server_base.clone(),
            self.replace_download_urls,
        );
        vanilla_generator.generate(output_dir, output_dir).await?;

        let forge_version =
            get_forge_version(&self.minecraft_version, &self.loader_version, &self.loader).await?;

        info!("Using {} version {}", self.loader, &forge_version);

        let forge_work_dir = work_dir
            .join(format!("{:?}", self.loader))
            .join(&get_full_version(&self.minecraft_version, &forge_version));
        let id = install_forge(
            &forge_work_dir,
            &get_java_dir(work_dir),
            &forge_version,
            &self.minecraft_version,
            &self.loader,
        )
        .await?;

        let versions_dir_from = forge_work_dir.join("versions");
        let versions_dir_to = get_versions_dir(output_dir);

        info!("Copying version metadata");
        let metadata_from = versions_dir_from.join(&id).join(format!("{}.json", id));
        let metadata_to = get_version_metadata_path(&versions_dir_to, &id);
        std::fs::copy(metadata_from, metadata_to)?;

        let mut forge_metadata = read_version_metadata(&versions_dir_to, &id).await?;

        let forge_libraries_dir = forge_work_dir.join("libraries");

        info!("Copying extra {} libs paths", self.loader);
        let metadata_libs_paths = forge_metadata
            .libraries
            .iter()
            .filter_map(|lib| {
                if let Some(downloads) = &lib.downloads {
                    if let Some(artifact) = &downloads.artifact {
                        if artifact.url != "" {
                            return lib.get_path(&forge_libraries_dir);
                        }
                    }
                }
                None
            })
            .collect::<HashSet<_>>();

        let extra_libs_paths_forge = get_files_in_dir(&forge_libraries_dir)
            .into_iter()
            .filter(|path| {
                let extension = path.extension().and_then(|ext| ext.to_str());
                path.is_file() && extension == Some("jar") && !metadata_libs_paths.contains(path)
            })
            .collect::<Vec<_>>();
        info!(
            "Found {} extra {} libs",
            extra_libs_paths_forge.len(),
            self.loader
        );
        debug!("Extra {} libs: {:?}", self.loader, extra_libs_paths_forge);

        // copy extra forge libs to output dir
        let libraries_dir = get_libraries_dir(&output_dir, &self.version_name);
        let extra_libs_paths = extra_libs_paths_forge
            .into_iter()
            .map(|lib_path| {
                let lib_path_relative = lib_path.strip_prefix(&forge_libraries_dir)?;
                let lib_dest = libraries_dir.join(lib_path_relative);
                std::fs::create_dir_all(lib_dest.parent().unwrap())?;
                std::fs::copy(&lib_path, &lib_dest)?;
                Ok(lib_dest)
            })
            .collect::<Result<Vec<_>, Box<dyn Error + Send + Sync>>>()?;

        if self.replace_download_urls {
            info!("Syncing version");
            sync_version(&forge_metadata, &self.version_name, output_dir).await?;

            replace_download_urls(
                &self.version_name,
                &mut forge_metadata,
                &self.download_server_base,
                output_dir,
            )
            .await?;

            save_version_metadata(&versions_dir_to, &forge_metadata).await?;
        }

        info!(
            "{} version \"{}\" generated",
            self.loader, self.version_name
        );

        Ok(GeneratorResult {
            id,
            extra_libs_paths,
        })
    }
}
