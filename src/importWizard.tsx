import { AlertTriangle, CheckCircle2, Download, Filter, Loader2, PackagePlus, Search, X } from "lucide-react";
import { useEffect, useMemo, useReducer, useState } from "react";

import type { LibraryModpack } from "./library";
import type { MessageKey } from "./i18n";
import {
  cancelCurseForgeImport,
  discoverCurseForgeReleases,
  importCurseForgeModpack,
  searchCurseForgeModpacks,
  type CurseForgeProject,
  type CurseForgeReleaseDiscovery,
  type CurseForgeReleaseSummary,
  type ImportProgress,
  type ImportedModpackResult,
} from "./tauri";
import "./importWizard.css";

export type { CurseForgeProject, CurseForgeReleaseSummary };

export type ReleaseFilters = {
  minecraftVersion: string;
  loader: string;
};

export type ImportWizardStatus = "idle" | "discovering" | "ready" | "downloading" | "success" | "failed";

export type ImportWizardState = {
  status: ImportWizardStatus;
  message: string;
  progress: ImportProgress | null;
  importedModpackId: number | null;
};

type ProjectSearchStatus = "idle" | "searching" | "ready" | "failed";

export type ImportWizardAction =
  | { type: "discovering" }
  | { type: "releaseReady" }
  | { type: "downloading" }
  | { type: "progress"; progress: ImportProgress }
  | { type: "downloadSucceeded"; result: ImportedModpackResult }
  | { type: "downloadFailed"; message: string }
  | { type: "downloadCancelled" };

export function createInitialImportWizardState(): ImportWizardState {
  return {
    status: "idle",
    message: "",
    progress: null,
    importedModpackId: null,
  };
}

export function importWizardReducer(
  state: ImportWizardState,
  action: ImportWizardAction,
): ImportWizardState {
  switch (action.type) {
    case "discovering":
      return { ...state, status: "discovering", message: "", progress: null };
    case "releaseReady":
      return { ...state, status: "ready", message: "", progress: null };
    case "downloading":
      return { ...state, status: "downloading", message: "", progress: { bytesDownloaded: 0, totalBytes: null } };
    case "progress":
      return { ...state, progress: action.progress };
    case "downloadSucceeded":
      return {
        ...state,
        status: "success",
        message: action.result.archivePath,
        progress: null,
        importedModpackId: action.result.modpackId,
      };
    case "downloadFailed":
      return { ...state, status: "failed", message: action.message, progress: null, importedModpackId: null };
    case "downloadCancelled":
      return {
        ...state,
        status: "failed",
        message: "Download cancelled. No imported modpack was added.",
        progress: null,
        importedModpackId: null,
      };
  }
}

export function getDefaultReleaseId(releases: CurseForgeReleaseSummary[]): number | null {
  return (
    [...releases].sort((left, right) => right.fileDate.localeCompare(left.fileDate))[0]?.fileId ??
    null
  );
}

export function getFilteredReleases(
  releases: CurseForgeReleaseSummary[],
  filters: ReleaseFilters,
): CurseForgeReleaseSummary[] {
  return releases.filter((release) => {
    const versionMatches =
      filters.minecraftVersion === "" || release.minecraftVersions.includes(filters.minecraftVersion);
    const loaderMatches = filters.loader === "" || release.loaders.includes(filters.loader);
    return versionMatches && loaderMatches;
  });
}

export function getNextSelectedReleaseId(
  currentFileId: number | null,
  visibleReleases: CurseForgeReleaseSummary[],
): number | null {
  if (currentFileId && visibleReleases.some((release) => release.fileId === currentFileId)) {
    return currentFileId;
  }

  return getDefaultReleaseId(visibleReleases);
}

export function isImportWizardBusy(status: ImportWizardStatus): boolean {
  return status === "discovering" || status === "downloading";
}

export function getDebouncedSearchQuery(query: string): string {
  return query.trim();
}

export function shouldSearchModpacks(query: string): boolean {
  return getDebouncedSearchQuery(query).length >= 2;
}

export function getProjectSourceUrl(project: CurseForgeProject): string {
  return `https://www.curseforge.com/minecraft/modpacks/${project.slug}`;
}

export function getProjectLogoUrl(project: CurseForgeProject): string | null {
  return project.logoUrl;
}

export function createInitialProjectSearchQuery(): string {
  return "";
}

function useDebouncedValue(value: string, delayMs: number): string {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const timeout = window.setTimeout(() => setDebouncedValue(value), delayMs);
    return () => window.clearTimeout(timeout);
  }, [delayMs, value]);

  return debouncedValue;
}

export function ImportWizardWorkspace(props: {
  onClose?: () => void;
  onImported: (library: LibraryModpack[], modpackId: number) => void;
  t: (key: MessageKey) => string;
}) {
  const { t } = props;
  const [state, dispatch] = useReducer(importWizardReducer, null, createInitialImportWizardState);
  const [query, setQuery] = useState(createInitialProjectSearchQuery);
  const debouncedQuery = useDebouncedValue(query, 450);
  const [projectSearchStatus, setProjectSearchStatus] = useState<ProjectSearchStatus>("idle");
  const [projectSearchMessage, setProjectSearchMessage] = useState("");
  const [projects, setProjects] = useState<CurseForgeProject[]>([]);
  const [selectedProject, setSelectedProject] = useState<CurseForgeProject | null>(null);
  const [discovery, setDiscovery] = useState<CurseForgeReleaseDiscovery | null>(null);
  const [selectedFileId, setSelectedFileId] = useState<number | null>(null);
  const [filters, setFilters] = useState<ReleaseFilters>({ minecraftVersion: "", loader: "" });
  const isDownloading = state.status === "downloading";
  const isBusy = isImportWizardBusy(state.status);

  const filteredReleases = useMemo(
    () => getFilteredReleases(discovery?.releases ?? [], filters),
    [discovery, filters],
  );
  const hasSelectedRelease = selectedFileId !== null && filteredReleases.length > 0;

  useEffect(() => {
    setSelectedFileId((current) => getNextSelectedReleaseId(current, filteredReleases));
  }, [filteredReleases]);

  useEffect(() => {
    const searchQuery = getDebouncedSearchQuery(debouncedQuery);
    let cancelled = false;

    if (!shouldSearchModpacks(searchQuery)) {
      setProjects([]);
      setProjectSearchStatus("idle");
      setProjectSearchMessage("");
      setSelectedProject(null);
      setDiscovery(null);
      setSelectedFileId(null);
      setFilters({ minecraftVersion: "", loader: "" });
      dispatch({ type: "releaseReady" });
      return;
    }

    setProjectSearchStatus("searching");
    setProjectSearchMessage("");
    setSelectedProject(null);
    setDiscovery(null);
    setSelectedFileId(null);
    setFilters({ minecraftVersion: "", loader: "" });
    dispatch({ type: "releaseReady" });

    searchCurseForgeModpacks(searchQuery)
      .then((nextProjects) => {
        if (cancelled) {
          return;
        }
        setProjects(nextProjects);
        setProjectSearchStatus("ready");
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }
        setProjects([]);
        setProjectSearchStatus("failed");
        setProjectSearchMessage(String(error));
      });

    return () => {
      cancelled = true;
    };
  }, [debouncedQuery]);

  async function handleSelectProject(project: CurseForgeProject) {
    setSelectedProject(project);
    setDiscovery(null);
    setSelectedFileId(null);
    setFilters({ minecraftVersion: "", loader: "" });
    dispatch({ type: "discovering" });
    try {
      const nextDiscovery = await discoverCurseForgeReleases(getProjectSourceUrl(project));
      setDiscovery(nextDiscovery);
      setSelectedFileId(nextDiscovery.defaultFileId);
      setFilters({ minecraftVersion: "", loader: "" });
      dispatch({ type: "releaseReady" });
    } catch (error) {
      dispatch({ type: "downloadFailed", message: String(error) });
    }
  }

  async function handleImport() {
    if (!selectedFileId || !discovery) {
      return;
    }

    dispatch({ type: "downloading" });
    try {
      const result = await importCurseForgeModpack(discovery.sourceUrl, selectedFileId, (progress) => {
        dispatch({ type: "progress", progress });
      });
      dispatch({ type: "downloadSucceeded", result });
      props.onImported(result.library, result.modpackId);
    } catch (error) {
      dispatch({ type: "downloadFailed", message: String(error) });
    }
  }

  async function handleCancel() {
    await cancelCurseForgeImport();
    dispatch({ type: "downloadCancelled" });
  }

  return (
    <section
      className={props.onClose ? "import-workspace import-modal" : "viewer-region import-workspace"}
      aria-label={t("workspace.addModpack")}
      role={props.onClose ? "dialog" : undefined}
    >
      {props.onClose ? (
        <header className="settings-modal-header import-modal-header">
          <div>
            <h2>{t("workspace.addModpack")}</h2>
            <span>{selectedProject?.name ?? t("import.searchSubtitle")}</span>
          </div>
          <button
            aria-label={t("settings.close")}
            className="icon-action"
            disabled={isDownloading}
            onClick={props.onClose}
            type="button"
          >
            <X size={18} />
          </button>
        </header>
      ) : (
        <div className="section-heading">
          <span>{t("workspace.addModpack")}</span>
          <strong>{selectedProject?.name ?? t("import.searchSubtitle")}</strong>
        </div>
      )}

      <div className="import-grid">
        <section className="import-panel">
          <label className="import-url-field import-search-field">
            <span>{t("import.searchLabel")}</span>
            <input
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder={t("import.searchPlaceholder")}
              value={query}
            />
          </label>
          <div className="project-search-status" aria-live="polite">
            {projectSearchStatus === "searching" ? (
              <>
                <Loader2 className="status-spinner" size={16} />
                {t("import.searchingModpacks")}
              </>
            ) : projectSearchStatus === "failed" ? (
              <>
                <AlertTriangle size={16} />
                {projectSearchMessage || t("import.searchFailed")}
              </>
            ) : shouldSearchModpacks(query) ? (
              <>
                <Search size={16} />
                {t("import.searchHelp")}
              </>
            ) : (
              <>
                <Search size={16} />
                {t("import.typeToSearch")}
              </>
            )}
          </div>

          <div className="project-list" aria-label={t("import.searchResults")}>
            {projects.map((project) => (
              <button
                className={
                  selectedProject?.id === project.id ? "project-row selected" : "project-row"
                }
                disabled={isDownloading}
                key={project.id}
                onClick={() => handleSelectProject(project)}
                type="button"
              >
                {getProjectLogoUrl(project) ? (
                  <img
                    alt=""
                    className="project-thumb"
                    loading="lazy"
                    src={getProjectLogoUrl(project) ?? undefined}
                  />
                ) : (
                  <span className="project-thumb placeholder">
                    <PackagePlus size={24} />
                  </span>
                )}
                <span className="project-row-copy">
                  <strong>{project.name}</strong>
                  <small>{project.slug}</small>
                </span>
              </button>
            ))}
            {projectSearchStatus === "ready" && projects.length === 0 && (
              <div className="project-list-empty">{t("import.noModpacksFound")}</div>
            )}
          </div>
        </section>

        <section className="import-panel release-list-panel">
          <div className="section-heading compact-heading">
            <span>
              {t("import.releases")}
              {selectedProject ? ` · ${selectedProject.name}` : ""}
            </span>
            <strong>{filteredReleases.length}</strong>
          </div>
          {discovery && (
            <div className="import-filters" aria-label={t("import.filters")}>
              <Filter size={16} />
              <select
                aria-label={t("import.minecraftVersion")}
                onChange={(event) => {
                  const minecraftVersion = event.currentTarget.value;
                  setFilters((current) => ({ ...current, minecraftVersion }));
                }}
                value={filters.minecraftVersion}
              >
                <option value="">{t("import.allMinecraftVersions")}</option>
                {discovery.minecraftVersions.map((version) => (
                  <option key={version} value={version}>
                    {version}
                  </option>
                ))}
              </select>
              <select
                aria-label={t("import.loader")}
                onChange={(event) => {
                  const loader = event.currentTarget.value;
                  setFilters((current) => ({ ...current, loader }));
                }}
                value={filters.loader}
              >
                <option value="">{t("import.allLoaders")}</option>
                {discovery.loaders.map((loader) => (
                  <option key={loader} value={loader}>
                    {loader}
                  </option>
                ))}
              </select>
            </div>
          )}
          {filteredReleases.length === 0 ? (
            <div className="empty-state-panel compact-empty">
              <PackagePlus size={28} />
              <h2>{selectedProject ? t("import.loadingReleasesTitle") : t("import.readyTitle")}</h2>
              <p>{state.message || t("import.readyBody")}</p>
            </div>
          ) : (
            <div className="release-list">
              {filteredReleases.map((release) => (
                <button
                  className={release.fileId === selectedFileId ? "release-row selected" : "release-row"}
                  key={release.fileId}
                  onClick={() => setSelectedFileId(release.fileId)}
                  type="button"
                >
                  <span>
                    <strong>{release.versionName}</strong>
                    <small>{release.fileName}</small>
                  </span>
                  <span>
                    <small>{release.minecraftVersions.join(", ") || t("library.unknown")}</small>
                    <small>{release.loaders.join(", ") || t("library.unknown")}</small>
                  </span>
                </button>
              ))}
            </div>
          )}

          <div className="import-status-row">
            {isBusy ? (
              <Loader2 className="status-spinner" size={17} />
            ) : state.status === "success" || hasSelectedRelease ? (
              <CheckCircle2 size={17} />
            ) : (
              <AlertTriangle size={17} />
            )}
            <span>{statusText(state, hasSelectedRelease, t)}</span>
          </div>
          {state.progress && (
            <progress
              className="import-progress"
              max={state.progress.totalBytes ?? 100}
              value={state.progress.bytesDownloaded}
            />
          )}
          {isDownloading && (
            <button className="secondary-action compact danger" onClick={handleCancel} type="button">
              <X size={16} />
              {t("import.cancel")}
            </button>
          )}
          <button
            className="primary-action compact"
            disabled={!selectedFileId || state.status === "downloading"}
            onClick={handleImport}
            type="button"
          >
            {isDownloading ? <Loader2 className="button-spinner" size={16} /> : <Download size={16} />}
            {isDownloading ? t("import.downloadingSelected") : t("import.downloadSelected")}
          </button>
        </section>
      </div>
    </section>
  );
}

export function statusText(
  state: ImportWizardState,
  hasSelectedRelease: boolean,
  t: (key: MessageKey) => string,
): string {
  if (state.status === "discovering") {
    return t("import.discovering");
  }
  if (state.status === "downloading") {
    return t("import.downloading");
  }
  if (state.status === "success") {
    return `${t("import.success")} ${state.message}`;
  }
  if (state.status === "failed" && state.message) {
    return state.message;
  }
  if (hasSelectedRelease) {
    return t("import.releaseReady");
  }
  return t("import.readyBody");
}
