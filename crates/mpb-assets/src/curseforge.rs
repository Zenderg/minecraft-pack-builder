use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use serde::{Deserialize, Serialize};

use crate::{AssetError, CancellationToken, CurseForgeGateway};

const CURSEFORGE_GAME_ID: u64 = 432;
const CURSEFORGE_MODPACK_CLASS_ID: u64 = 4471;
const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const CURSEFORGE_SEARCH_INDEX: u64 = 0;
const CURSEFORGE_SEARCH_PAGE_SIZE: u64 = 25;
const CURSEFORGE_SEARCH_SORT_FIELD_FEATURED: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedModpackUrl {
    pub slug: String,
    pub normalized_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeProject {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeRelease {
    pub file_id: u64,
    pub display_name: String,
    pub file_name: String,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub file_date: String,
    pub file_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSummary {
    pub file_id: u64,
    pub version_name: String,
    pub file_name: String,
    pub minecraft_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub file_date: String,
    pub file_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredReleases {
    pub modpack: CurseForgeProject,
    pub source_url: String,
    pub releases: Vec<ReleaseSummary>,
    pub minecraft_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub default_file_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseFilter {
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedArchive {
    pub path: PathBuf,
    pub bytes_downloaded: u64,
}

pub fn search_modpack_projects(
    gateway: &impl CurseForgeGateway,
    api_key: &str,
    query: &str,
) -> Result<Vec<CurseForgeProject>, AssetError> {
    let trimmed = query.trim();
    if trimmed.len() < 2 {
        return Ok(Vec::new());
    }

    gateway.search_modpack_projects(api_key, trimmed)
}

pub fn parse_modpack_page_url(value: &str) -> Result<ParsedModpackUrl, AssetError> {
    let trimmed = value.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .ok_or(AssetError::UnsupportedUrl)?;
    let (host, path_with_query) = without_scheme
        .split_once('/')
        .ok_or(AssetError::UnsupportedUrl)?;

    if host != "www.curseforge.com" && host != "curseforge.com" {
        return Err(AssetError::UnsupportedUrl);
    }

    let path = path_with_query
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "minecraft" || parts[1] != "modpacks" {
        return Err(AssetError::UnsupportedUrl);
    }

    let slug = parts[2].trim();
    if slug.is_empty() {
        return Err(AssetError::UnsupportedUrl);
    }

    Ok(ParsedModpackUrl {
        slug: slug.to_string(),
        normalized_url: format!("https://www.curseforge.com/minecraft/modpacks/{slug}"),
    })
}

pub fn discover_modpack_releases(
    gateway: &impl CurseForgeGateway,
    api_key: &str,
    page_url: &str,
) -> Result<DiscoveredReleases, AssetError> {
    let parsed = parse_modpack_page_url(page_url)?;
    let modpack = gateway
        .find_modpack_project(api_key, &parsed.slug)?
        .ok_or_else(|| AssetError::ModpackNotFound {
            slug: parsed.slug.clone(),
        })?;
    let mut files = gateway.list_project_files(api_key, modpack.id)?;
    files.sort_by(|left, right| right.file_date.cmp(&left.file_date));

    let releases = files
        .iter()
        .map(release_to_summary)
        .collect::<Vec<ReleaseSummary>>();
    let default_file_id = releases
        .first()
        .map(|release| release.file_id)
        .ok_or_else(|| AssetError::Api("modpack has no downloadable releases".to_string()))?;
    let minecraft_versions = unique_ordered_versions(
        releases
            .iter()
            .flat_map(|release| release.minecraft_versions.iter().cloned()),
    );
    let mut loaders = unique_ordered_versions(
        releases
            .iter()
            .flat_map(|release| release.loaders.iter().cloned()),
    );
    loaders.sort();

    Ok(DiscoveredReleases {
        modpack,
        source_url: parsed.normalized_url,
        releases,
        minecraft_versions,
        loaders,
        default_file_id,
    })
}

pub fn filter_releases<'a>(
    releases: &'a [ReleaseSummary],
    filter: &ReleaseFilter,
) -> Vec<&'a ReleaseSummary> {
    releases
        .iter()
        .filter(|release| {
            filter
                .minecraft_version
                .as_ref()
                .is_none_or(|version| release.minecraft_versions.contains(version))
        })
        .filter(|release| {
            filter
                .loader
                .as_ref()
                .is_none_or(|loader| release.loaders.contains(loader))
        })
        .collect()
}

pub fn download_release_archive(
    gateway: &impl CurseForgeGateway,
    api_key: &str,
    release: &CurseForgeRelease,
    destination: &Path,
    cancellation: &CancellationToken,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<DownloadedArchive, AssetError> {
    if cancellation.is_cancelled() {
        return Err(AssetError::Cancelled);
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut reader = gateway.open_download(api_key, release)?;
    let temporary_path = destination.with_extension("download");
    let mut file = File::create(&temporary_path)?;
    let mut buffer = [0_u8; 16 * 1024];
    let mut bytes_downloaded = 0_u64;

    loop {
        if cancellation.is_cancelled() {
            drop(file);
            let _ = std::fs::remove_file(&temporary_path);
            let _ = std::fs::remove_file(destination);
            return Err(AssetError::Cancelled);
        }

        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        bytes_downloaded += read as u64;
        on_progress(DownloadProgress {
            bytes_downloaded,
            total_bytes: Some(release.file_length),
        });
    }

    drop(file);
    std::fs::rename(&temporary_path, destination)?;
    Ok(DownloadedArchive {
        path: destination.to_path_buf(),
        bytes_downloaded,
    })
}


fn release_to_summary(release: &CurseForgeRelease) -> ReleaseSummary {
    ReleaseSummary {
        file_id: release.file_id,
        version_name: release.display_name.clone(),
        file_name: release.file_name.clone(),
        minecraft_versions: release
            .game_versions
            .iter()
            .filter(|version| looks_like_minecraft_version(version))
            .cloned()
            .collect(),
        loaders: release
            .game_versions
            .iter()
            .filter(|version| looks_like_loader(version))
            .cloned()
            .collect(),
        file_date: release.file_date.clone(),
        file_length: release.file_length,
    }
}

fn looks_like_minecraft_version(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
}

fn looks_like_loader(value: &str) -> bool {
    matches!(value, "Forge" | "NeoForge" | "Fabric" | "Quilt")
}

fn unique_ordered_versions(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .filter(|value| seen.insert(value.clone()))
        .collect::<Vec<_>>()
}

#[derive(Debug, Clone)]
pub struct CurseForgeHttpGateway {
    client: Client,
}

impl CurseForgeHttpGateway {
    pub fn new() -> Result<Self, AssetError> {
        let client = Client::builder()
            .user_agent("MinecraftPackBuilder/0.1")
            .build()
            .map_err(|error| AssetError::Http(error.to_string()))?;
        Ok(Self { client })
    }

    fn api_headers(api_key: &str) -> Result<HeaderMap, AssetError> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(api_key.trim())
                .map_err(|error| AssetError::Http(error.to_string()))?,
        );
        Ok(headers)
    }
}

fn modpack_search_query_params(query: &str) -> Vec<(&'static str, String)> {
    vec![
        ("gameId", CURSEFORGE_GAME_ID.to_string()),
        ("classId", CURSEFORGE_MODPACK_CLASS_ID.to_string()),
        ("searchFilter", query.to_string()),
        ("index", CURSEFORGE_SEARCH_INDEX.to_string()),
        ("pageSize", CURSEFORGE_SEARCH_PAGE_SIZE.to_string()),
        (
            "sortField",
            CURSEFORGE_SEARCH_SORT_FIELD_FEATURED.to_string(),
        ),
        ("sortOrder", "desc".to_string()),
    ]
}

impl CurseForgeGateway for CurseForgeHttpGateway {
    fn search_modpack_projects(
        &self,
        api_key: &str,
        query: &str,
    ) -> Result<Vec<CurseForgeProject>, AssetError> {
        let response = self
            .client
            .get(format!("{CURSEFORGE_API_BASE}/mods/search"))
            .headers(Self::api_headers(api_key)?)
            .query(&modpack_search_query_params(query))
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .json::<CurseForgeSearchResponse>()
            .map_err(|error| AssetError::Api(error.to_string()))?;

        Ok(response
            .data
            .into_iter()
            .map(|modpack| CurseForgeProject {
                id: modpack.id,
                name: modpack.name,
                slug: modpack.slug,
                logo_url: modpack
                    .logo
                    .and_then(|logo| logo.thumbnail_url.or(logo.url)),
            })
            .collect())
    }

    fn find_modpack_project(
        &self,
        api_key: &str,
        slug: &str,
    ) -> Result<Option<CurseForgeProject>, AssetError> {
        let response = self
            .client
            .get(format!("{CURSEFORGE_API_BASE}/mods/search"))
            .headers(Self::api_headers(api_key)?)
            .query(&[
                ("gameId", CURSEFORGE_GAME_ID.to_string()),
                ("classId", CURSEFORGE_MODPACK_CLASS_ID.to_string()),
                ("slug", slug.to_string()),
            ])
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .json::<CurseForgeSearchResponse>()
            .map_err(|error| AssetError::Api(error.to_string()))?;

        Ok(response
            .data
            .into_iter()
            .next()
            .map(|modpack| CurseForgeProject {
                id: modpack.id,
                name: modpack.name,
                slug: modpack.slug,
                logo_url: modpack
                    .logo
                    .and_then(|logo| logo.thumbnail_url.or(logo.url)),
            }))
    }

    fn list_project_files(
        &self,
        api_key: &str,
        project_id: u64,
    ) -> Result<Vec<CurseForgeRelease>, AssetError> {
        let response = self
            .client
            .get(format!("{CURSEFORGE_API_BASE}/mods/{project_id}/files"))
            .headers(Self::api_headers(api_key)?)
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .json::<CurseForgeFilesResponse>()
            .map_err(|error| AssetError::Api(error.to_string()))?;

        Ok(response
            .data
            .into_iter()
            .map(|file| CurseForgeRelease {
                file_id: file.id,
                display_name: file.display_name,
                file_name: file.file_name,
                download_url: file.download_url,
                game_versions: file.game_versions,
                file_date: file.file_date,
                file_length: file.file_length,
            })
            .collect())
    }

    fn open_download(
        &self,
        api_key: &str,
        release: &CurseForgeRelease,
    ) -> Result<Box<dyn Read>, AssetError> {
        let url = release
            .download_url
            .as_ref()
            .ok_or(AssetError::MissingDownloadUrl {
                file_id: release.file_id,
            })?;
        let response = self
            .client
            .get(url)
            .headers(Self::api_headers(api_key)?)
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?;
        Ok(Box::new(response))
    }

    fn open_mod_file_download(
        &self,
        api_key: &str,
        _project_id: u64,
        file_id: u64,
    ) -> Result<Box<dyn Read>, AssetError> {
        let response = self
            .client
            .post(format!("{CURSEFORGE_API_BASE}/mods/files"))
            .headers(Self::api_headers(api_key)?)
            .json(&CurseForgeFilesRequest {
                file_ids: vec![file_id],
            })
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .json::<CurseForgeFilesResponse>()
            .map_err(|error| AssetError::Api(error.to_string()))?;

        let download_url = response
            .data
            .into_iter()
            .find(|file| file.id == file_id)
            .and_then(|file| file.download_url)
            .ok_or(AssetError::MissingDownloadUrl { file_id })?;
        let response = self
            .client
            .get(download_url)
            .headers(Self::api_headers(api_key)?)
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?;
        Ok(Box::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modpack_search_matches_prismlauncher_default_search_params() {
        let params = modpack_search_query_params("create");

        assert!(params.contains(&("index", "0".to_string())));
        assert!(params.contains(&("pageSize", "25".to_string())));
        assert!(params.contains(&("searchFilter", "create".to_string())));
        assert!(params.contains(&("sortField", "1".to_string())));
        assert!(params.contains(&("sortOrder", "desc".to_string())));
        assert!(!params.iter().any(|(key, _)| *key == "page"));
        assert!(!params.iter().any(|(key, _)| *key == "sortBy"));
    }

    #[test]
    fn curseforge_files_request_uses_api_file_ids_field() {
        let json = serde_json::to_value(CurseForgeFilesRequest {
            file_ids: vec![8054109],
        })
        .expect("serialize files request");

        assert_eq!(json, serde_json::json!({ "fileIds": [8054109] }));
    }
}

#[derive(Debug, Deserialize)]
struct CurseForgeSearchResponse {
    data: Vec<CurseForgeModDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeModDto {
    id: u64,
    name: String,
    slug: String,
    logo: Option<CurseForgeLogoDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeLogoDto {
    thumbnail_url: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeFilesResponse {
    data: Vec<CurseForgeFileDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeFilesRequest {
    file_ids: Vec<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeFileDto {
    id: u64,
    display_name: String,
    file_name: String,
    download_url: Option<String>,
    game_versions: Vec<String>,
    file_date: String,
    file_length: u64,
}
