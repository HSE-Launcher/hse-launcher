use std::{
    collections::{HashMap, HashSet},
    error::Error,
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

async fn download_forge_installer(
    full_version: &str,
    work_dir: &Path,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let filename = format!("forge-{}-installer.jar", full_version);
    let forge_installer_url = format!("{}{}/{}", FORGE_INSTALLER_BASE_URL, full_version, filename);
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

pub struct ForgeGenerator {
    version_name: String,
    minecraft_version: String,
    loader_version: Option<String>,
    download_server_base: String,
    replace_download_urls: bool,
}

impl ForgeGenerator {
    pub fn new(
        version_name: String,
        minecraft_version: String,
        loader_version: Option<String>,
        download_server_base: String,
        replace_download_urls: bool,
    ) -> Self {
        Self {
            version_name,
            minecraft_version,
            loader_version,
            download_server_base,
            replace_download_urls,
        }
    }
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

#[derive(thiserror::Error, Debug)]
pub enum ForgeError {
    #[error("Forge version {0} not found for minecraft {1}")]
    ForgeVersionNotFound(String, String),
    #[error("No forge profiles found")]
    NoForgeProfiles,
}

#[async_trait]
impl VersionGenerator for ForgeGenerator {
    async fn generate(
        &self,
        output_dir: &Path,
        work_dir: &Path,
    ) -> Result<GeneratorResult, Box<dyn Error + Send + Sync>> {
        info!(
            "Generating forge modpack \"{}\", minecraft version {}",
            self.version_name, self.minecraft_version
        );

        info!("Generating vanilla version first");
        let vanilla_generator = VanillaGenerator::new(
            self.version_name.clone(),
            self.minecraft_version.clone(),
            self.download_server_base.clone(),
            self.replace_download_urls,
        );
        vanilla_generator.generate(output_dir, output_dir).await?;

        let loader_version = match &self.loader_version {
            Some(loader_version) => loader_version.clone(),
            None => {
                let version = "recommended".to_string();
                info!(
                    "Loader version not specified, using version \"{}\"",
                    version
                );
                version
            }
        };
        let forge_version = match loader_version.as_str() {
            "latest" | "recommended" => {
                info!(
                    "Getting {} forge version for minecraft {}",
                    loader_version, &self.minecraft_version
                );
                let promotions = ForgePromotions::from_url(FORGE_PROMOTIONS_URL).await?;
                promotions
                    .get_latest_version(&self.minecraft_version, &loader_version)
                    .ok_or(Box::new(ForgeError::ForgeVersionNotFound(
                        loader_version,
                        self.minecraft_version.clone(),
                    )))?
            }

            other => other.to_string(),
        };
        let forge_maven_metadata = ForgeMavenMetadata::from_url(FORGE_MAVEN_METADATA_URL).await?;
        if !forge_maven_metadata.has_version(&self.minecraft_version, &forge_version) {
            return Err(ForgeError::ForgeVersionNotFound(
                forge_version,
                self.minecraft_version.clone(),
            )
            .into());
        }
        info!("Using forge version {}", &forge_version);

        info!("Getting vanilla java version");
        let java_version = get_vanilla_java_version(&self.minecraft_version)
            .await?
            .map_or_else(
                || {
                    warn!("Java version not found, using default");
                    "8".to_string()
                },
                |v| v,
            );

        info!("Getting java {}", &java_version);
        let java_dir = get_java_dir(work_dir);
        let java_installation;
        if let Some(existing_java_installation) = get_java(&java_version, &java_dir) {
            java_installation = existing_java_installation;
        } else {
            info!("Java installation not found, downloading");

            let progress_bar = Arc::new(TerminalProgressBar::new());

            progress_bar.set_message("Downloading java...");
            java_installation = download_java(&java_version, &java_dir, progress_bar).await?;
        }

        let full_version = format!("{}-{}", self.minecraft_version, forge_version);
        let forge_work_dir = work_dir.join(format!("forge-{}", &full_version));

        let forge_installer_path = download_forge_installer(&full_version, &forge_work_dir).await?;
        info!("Downloaded forge installer");

        // trick forge installer into thinking that the folder is actually the minecraft folder
        std::fs::create_dir_all(
            forge_work_dir
                .join("versions")
                .join(&self.minecraft_version),
        )?;
        let mut file = std::fs::File::create(forge_work_dir.join("launcher_profiles.json"))?;
        file.write(b"{}")?;

        info!("Running forge installer");
        exec_custom_command_in_dir(
            &format!(
                "{} -jar {} --installClient",
                to_abs_path_str(&java_installation.path)?,
                to_abs_path_str(&forge_installer_path)?,
            ),
            &forge_work_dir,
        )
        .await?;

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

        let versions_dir_from = forge_work_dir.join("versions");
        let versions_dir_to = get_versions_dir(output_dir);

        info!("Copying version metadata");
        let metadata_from = versions_dir_from.join(&id).join(format!("{}.json", id));
        let metadata_to = get_version_metadata_path(&versions_dir_to, &id);
        std::fs::copy(metadata_from, metadata_to)?;

        let mut forge_metadata = read_version_metadata(&versions_dir_to, &id).await?;

        let forge_libraries_dir = forge_work_dir.join("libraries");

        info!("Copying extra forge libs paths");
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
        info!("Found {} extra forge libs", extra_libs_paths_forge.len());
        debug!("Extra forge libs: {:?}", extra_libs_paths_forge);

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

        info!("Forge version \"{}\" generated", self.version_name);

        Ok(GeneratorResult {
            id,
            extra_libs_paths,
        })
    }
}
