use std::io::{Cursor, Read};
use std::path::Path;

use mpb_assets::{
    discover_modpack_releases, download_release_archive, filter_releases, parse_modpack_page_url,
    search_modpack_projects, CancellationToken, CurseForgeGateway, CurseForgeProject,
    CurseForgeRelease, DownloadProgress, ReleaseFilter,
};
use tempfile::tempdir;

#[test]
fn parses_curseforge_modpack_page_urls_and_rejects_other_pages() {
    let parsed = parse_modpack_page_url("https://www.curseforge.com/minecraft/modpacks/aoc")
        .expect("parse modpack page");

    assert_eq!(parsed.slug, "aoc");
    assert_eq!(
        parsed.normalized_url,
        "https://www.curseforge.com/minecraft/modpacks/aoc"
    );

    assert!(parse_modpack_page_url("https://www.curseforge.com/minecraft/mc-mods/create").is_err());
    assert!(parse_modpack_page_url("https://example.com/minecraft/modpacks/aoc").is_err());
}

#[test]
fn release_discovery_maps_curseforge_files_and_selects_latest_by_date() {
    let gateway = FakeGateway::default();

    let discovered = discover_modpack_releases(
        &gateway,
        "test-key",
        "https://www.curseforge.com/minecraft/modpacks/aoc",
    )
    .expect("discover releases");

    assert_eq!(discovered.modpack.slug, "aoc");
    assert_eq!(discovered.releases.len(), 3);
    assert_eq!(discovered.default_file_id, 300);
    assert_eq!(discovered.minecraft_versions, vec!["1.20.1", "1.19.2"]);
    assert_eq!(discovered.loaders, vec!["Forge", "NeoForge"]);
}

#[test]
fn modpack_search_returns_matching_projects_for_a_query() {
    let gateway = FakeGateway::default();

    let projects =
        search_modpack_projects(&gateway, "test-key", " all of create ").expect("search modpacks");

    assert_eq!(
        projects,
        vec![CurseForgeProject {
            id: 42,
            name: "AOC".to_string(),
            slug: "aoc".to_string(),
            logo_url: Some("https://images.example/aoc-thumb.png".to_string()),
        }]
    );
    assert!(search_modpack_projects(&gateway, "test-key", " ")
        .unwrap()
        .is_empty());
}

#[test]
fn release_filters_match_minecraft_version_and_loader() {
    let discovered = discover_modpack_releases(
        &FakeGateway::default(),
        "test-key",
        "https://www.curseforge.com/minecraft/modpacks/aoc",
    )
    .expect("discover releases");

    let filtered = filter_releases(
        &discovered.releases,
        &ReleaseFilter {
            minecraft_version: Some("1.20.1".to_string()),
            loader: Some("NeoForge".to_string()),
        },
    );

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].file_id, 300);
}

#[test]
fn downloads_archive_with_progress_and_cancellation() {
    let gateway = FakeGateway::default();
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("aoc.zip");
    let mut progress = Vec::new();

    let downloaded = download_release_archive(
        &gateway,
        "test-key",
        &gateway.releases[0],
        &archive_path,
        &CancellationToken::new(),
        |event| progress.push(event),
    )
    .expect("download archive");

    assert_eq!(downloaded.path, archive_path);
    assert_eq!(downloaded.bytes_downloaded, 12);
    assert_eq!(
        std::fs::read(&downloaded.path).expect("archive"),
        b"modpack-data"
    );
    assert!(progress.iter().any(|event| event.bytes_downloaded == 12));

    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let cancelled_path = temp.path().join("cancelled.zip");
    let error = download_release_archive(
        &gateway,
        "test-key",
        &gateway.releases[0],
        &cancelled_path,
        &cancelled,
        |_| {},
    )
    .expect_err("cancelled download should fail");

    assert_eq!(error.to_string(), "download was cancelled");
    assert!(!cancelled_path.exists());
}

struct FakeGateway {
    project: CurseForgeProject,
    releases: Vec<CurseForgeRelease>,
}

impl Default for FakeGateway {
    fn default() -> Self {
        Self {
            project: CurseForgeProject {
                id: 42,
                name: "AOC".to_string(),
                slug: "aoc".to_string(),
                logo_url: Some("https://images.example/aoc-thumb.png".to_string()),
            },
            releases: vec![
                release(
                    100,
                    "AOC 1.0.0",
                    "2026-01-01T00:00:00Z",
                    &["1.19.2", "Forge"],
                ),
                release(
                    200,
                    "AOC 1.1.0",
                    "2026-02-01T00:00:00Z",
                    &["1.20.1", "Forge"],
                ),
                release(
                    300,
                    "AOC 1.2.0",
                    "2026-03-01T00:00:00Z",
                    &["1.20.1", "NeoForge"],
                ),
            ],
        }
    }
}

impl CurseForgeGateway for FakeGateway {
    fn search_modpack_projects(
        &self,
        _api_key: &str,
        query: &str,
    ) -> Result<Vec<CurseForgeProject>, mpb_assets::AssetError> {
        Ok(query
            .contains("create")
            .then(|| self.project.clone())
            .into_iter()
            .collect())
    }

    fn find_modpack_project(
        &self,
        _api_key: &str,
        slug: &str,
    ) -> Result<Option<CurseForgeProject>, mpb_assets::AssetError> {
        Ok((slug == self.project.slug).then(|| self.project.clone()))
    }

    fn list_project_files(
        &self,
        _api_key: &str,
        _project_id: u64,
    ) -> Result<Vec<CurseForgeRelease>, mpb_assets::AssetError> {
        Ok(self.releases.clone())
    }

    fn open_download(
        &self,
        _api_key: &str,
        _release: &CurseForgeRelease,
    ) -> Result<Box<dyn Read>, mpb_assets::AssetError> {
        Ok(Box::new(Cursor::new(b"modpack-data".to_vec())))
    }
}

fn release(
    file_id: u64,
    display_name: &str,
    file_date: &str,
    game_versions: &[&str],
) -> CurseForgeRelease {
    CurseForgeRelease {
        file_id,
        display_name: display_name.to_string(),
        file_name: format!("{display_name}.zip"),
        download_url: Some(format!("https://files.example/{file_id}.zip")),
        game_versions: game_versions
            .iter()
            .map(|value| value.to_string())
            .collect(),
        file_date: file_date.to_string(),
        file_length: 12,
    }
}

#[allow(dead_code)]
fn _assert_progress_is_send(_: DownloadProgress, _: &Path) {}
