import {
  AlertTriangle,
  Download,
  Info,
  Layers3,
  Loader2,
  MoreHorizontal,
  Pencil,
  Plus,
  Trash2,
} from "lucide-react";
import { useEffect, useState } from "react";
import type React from "react";

import {
  compactLibraryNodeGap,
  getLoaderIconKind,
  getModpackMenuPlacement,
  getNextOpenModpackMenuId,
  type LoaderIconKind,
  type LibraryModpack,
  type LibraryScheme,
  type LibrarySelection,
} from "../library";
import type { Translator } from "./types";

export function LibraryTree(props: {
  expandedModpackIds: Set<number>;
  library: LibraryModpack[];
  onCreateScheme: (modpackId: number) => void;
  onDeleteModpack: (modpack: LibraryModpack) => void;
  onDeleteScheme: (scheme: LibraryScheme) => void;
  onExportScheme: (scheme: LibraryScheme) => void;
  onRenameModpack: (modpack: LibraryModpack) => void;
  onRenameScheme: (scheme: LibraryScheme) => void;
  onSelect: (selection: LibrarySelection) => void;
  onShowImportJob: (modpack: LibraryModpack) => void;
  onShowModpackInfo: (modpack: LibraryModpack) => void;
  onToggleModpack: (modpackId: number) => void;
  selected: LibrarySelection | null;
  t: Translator;
}) {
  const { t } = props;
  const [openLibraryMenu, setOpenLibraryMenu] = useState<{
    kind: "modpack" | "scheme";
    id: number;
    left: number;
    top: number;
  } | null>(null);
  useEffect(() => {
    if (openLibraryMenu === null) {
      return;
    }

    const closeMenu = () => {
      setOpenLibraryMenu(null);
    };

    window.addEventListener("pointerdown", closeMenu);
    return () => window.removeEventListener("pointerdown", closeMenu);
  }, [openLibraryMenu]);

  if (props.library.length === 0) {
    return (
      <div className="library-empty-state">
        <p>{t("library.empty")}</p>
      </div>
    );
  }

  return (
    <div
      className="library-tree"
      style={{ "--library-node-gap": `${compactLibraryNodeGap}px` } as React.CSSProperties}
    >
      {props.library.map((modpack) => (
        <div
          className={props.expandedModpackIds.has(modpack.id) ? "library-node expanded" : "library-node"}
          key={modpack.id}
        >
          <div className="tree-item modpack-row">
            <button
              className="tree-label modpack-label"
              onClick={() => {
                if (modpack.importStatus === "imported") {
                  props.onToggleModpack(modpack.id);
                } else {
                  props.onShowImportJob(modpack);
                }
              }}
              type="button"
            >
              <LoaderIcon kind={getLoaderIconKind(modpack.loader)} />
              <span title={modpack.localName}>{modpack.localName}</span>
            </button>
            {modpack.importStatus === "imported" ? (
              <div className="tree-actions">
                <button
                  aria-label={t("library.createScheme")}
                  className="icon-action small"
                  onClick={() => props.onCreateScheme(modpack.id)}
                  type="button"
                >
                  <Plus size={14} />
                </button>
                <button
                  aria-label={t("library.modpackActions")}
                  className="icon-action small"
                  onClick={(event) => {
                    event.stopPropagation();
                    const placement = getModpackMenuPlacement(event.currentTarget.getBoundingClientRect(), {
                      width: window.innerWidth,
                      height: window.innerHeight,
                    });
                    setOpenLibraryMenu((current) => {
                      const openModpackId = current?.kind === "modpack" ? current.id : null;
                      const nextId = getNextOpenModpackMenuId(openModpackId, modpack.id, "menuButton");
                      return nextId === null ? null : { kind: "modpack", id: nextId, ...placement };
                    });
                  }}
                  type="button"
                >
                  <MoreHorizontal size={15} />
                </button>
              </div>
            ) : (
              <ImportStatusIndicator status={modpack.importStatus} t={t} />
            )}
            {openLibraryMenu?.kind === "modpack" && openLibraryMenu.id === modpack.id && (
              <div
                className="modpack-menu"
                onPointerDown={(event) => event.stopPropagation()}
                role="menu"
                style={
                  {
                    "--modpack-menu-left": `${openLibraryMenu.left}px`,
                    "--modpack-menu-top": `${openLibraryMenu.top}px`,
                  } as React.CSSProperties
                }
              >
                <button
                  onClick={() => {
                    setOpenLibraryMenu(null);
                    props.onShowModpackInfo(modpack);
                  }}
                  role="menuitem"
                  type="button"
                >
                  <Info size={14} />
                  {t("library.information")}
                </button>
                <button
                  onClick={() => {
                    setOpenLibraryMenu(null);
                    props.onRenameModpack(modpack);
                  }}
                  role="menuitem"
                  type="button"
                >
                  <Pencil size={14} />
                  {t("library.menuRename")}
                </button>
                <button
                  className="danger"
                  onClick={() => {
                    setOpenLibraryMenu(null);
                    props.onDeleteModpack(modpack);
                  }}
                  role="menuitem"
                  type="button"
                >
                  <Trash2 size={14} />
                  {t("library.menuDelete")}
                </button>
              </div>
            )}
          </div>
          {modpack.importStatus === "imported" &&
            props.expandedModpackIds.has(modpack.id) &&
            (modpack.schemes.length === 0 ? (
              <div className="tree-item nested empty-scheme-row">{t("library.noSchemes")}</div>
            ) : (
              modpack.schemes.map((scheme) => (
                <div
                  className={
                    props.selected?.schemeId === scheme.id
                      ? "tree-item nested selected scheme-row"
                      : "tree-item nested scheme-row"
                  }
                  key={scheme.id}
                >
                  <button
                    className="tree-label scheme-label"
                    onClick={() => props.onSelect({ modpackId: modpack.id, schemeId: scheme.id })}
                    type="button"
                  >
                    <Layers3 size={15} />
                    <span title={scheme.name}>{scheme.name}</span>
                  </button>
                  <div className="tree-actions">
                    <button
                      aria-label={t("library.schemeActions")}
                      className="icon-action small"
                      onClick={(event) => {
                        event.stopPropagation();
                        const placement = getModpackMenuPlacement(event.currentTarget.getBoundingClientRect(), {
                          width: window.innerWidth,
                          height: window.innerHeight,
                        });
                        setOpenLibraryMenu((current) =>
                          current?.kind === "scheme" && current.id === scheme.id
                            ? null
                            : { kind: "scheme", id: scheme.id, ...placement },
                        );
                      }}
                      type="button"
                    >
                      <MoreHorizontal size={15} />
                    </button>
                  </div>
                  {openLibraryMenu?.kind === "scheme" && openLibraryMenu.id === scheme.id && (
                    <div
                      className="modpack-menu"
                      onPointerDown={(event) => event.stopPropagation()}
                      role="menu"
                      style={
                        {
                          "--modpack-menu-left": `${openLibraryMenu.left}px`,
                          "--modpack-menu-top": `${openLibraryMenu.top}px`,
                        } as React.CSSProperties
                      }
                    >
                      <button
                        onClick={() => {
                          setOpenLibraryMenu(null);
                          props.onRenameScheme(scheme);
                        }}
                        role="menuitem"
                        type="button"
                      >
                        <Pencil size={14} />
                        {t("library.menuRename")}
                      </button>
                      <button
                        onClick={() => {
                          setOpenLibraryMenu(null);
                          props.onExportScheme(scheme);
                        }}
                        role="menuitem"
                        type="button"
                      >
                        <Download size={14} />
                        {t("library.menuExport")}
                      </button>
                      <button
                        className="danger"
                        onClick={() => {
                          setOpenLibraryMenu(null);
                          props.onDeleteScheme(scheme);
                        }}
                        role="menuitem"
                        type="button"
                      >
                        <Trash2 size={14} />
                        {t("library.menuDelete")}
                      </button>
                    </div>
                  )}
                </div>
              ))
            ))}
        </div>
      ))}
    </div>
  );
}

function ImportStatusIndicator({
  status,
  t,
}: {
  status: LibraryModpack["importStatus"];
  t: Translator;
}) {
  if (status === "importing") {
    return (
      <span className="import-status-indicator importing" title={t("import.state.importing")}>
        <Loader2 className="status-spinner" size={15} />
      </span>
    );
  }
  if (status === "failed") {
    return (
      <span className="import-status-indicator failed" title={t("import.state.failed")}>
        <AlertTriangle size={15} />
      </span>
    );
  }
  return null;
}

function LoaderIcon({ kind }: { kind: LoaderIconKind }) {
  const label = {
    forge: "F",
    neoforge: "NF",
    fabric: "Fb",
    quilt: "Q",
    generic: "MC",
  }[kind];

  return (
    <span aria-hidden="true" className={`loader-icon ${kind}`}>
      {label}
    </span>
  );
}
