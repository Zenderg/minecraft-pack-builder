export type ImportStatus = "imported" | "importing" | "failed";

export type SchemeDimensions = [number, number, number];

export type LibraryScheme = {
  id: number;
  modpackId: number;
  name: string;
  dimensions: SchemeDimensions;
};

export type LibraryModpack = {
  id: number;
  localName: string;
  sourceUrl: string | null;
  versionName: string;
  minecraftVersion: string | null;
  loader: string | null;
  importStatus: ImportStatus;
  importMessage: string | null;
  schemes: LibraryScheme[];
};

export type LibrarySelection = {
  modpackId: number;
  schemeId: number;
};

export type SchemeDraft = {
  modpackId: number;
  name: string;
  dimensions: SchemeDimensions;
};

export const sidebarWidthLimits = {
  min: 248,
  max: 420,
  default: 276,
} as const;

export const compactLibraryNodeGap = 2;
export const modpackMenuSize = {
  width: 168,
  height: 110,
  viewportPadding: 8,
} as const;

export type LoaderIconKind = "forge" | "neoforge" | "fabric" | "quilt" | "generic";
export type ModpackMenuPointerTarget = "menuButton" | "menuSurface" | "outside";
export type LibraryDialogKind =
  | "createScheme"
  | "renameScheme"
  | "renameModpack"
  | "infoModpack"
  | "deleteScheme"
  | "deleteModpack";
export type LibraryDialogContent = {
  titleKey:
    | "library.createSchemeTitle"
    | "library.renameSchemeTitle"
    | "library.renameModpackTitle"
    | "library.informationTitle"
    | "library.deleteSchemeTitle"
    | "library.deleteModpackTitle";
  bodyKey?: "library.informationDescription" | "library.confirmDeleteScheme" | "library.confirmDeleteModpack";
  fieldKey?: "library.nameLabel";
  tone: "form" | "info" | "danger";
};
export type RectLike = {
  left: number;
  right: number;
  top: number;
  bottom: number;
};
export type ViewportSize = {
  width: number;
  height: number;
};

export function createEmptyLibraryDraft(modpackId: number): SchemeDraft {
  return {
    modpackId,
    name: "New scheme",
    dimensions: [64, 64, 64],
  };
}

export function clampSidebarWidth(width: number): number {
  return Math.min(sidebarWidthLimits.max, Math.max(sidebarWidthLimits.min, Math.round(width)));
}

export function getLoaderIconKind(loader: string | null): LoaderIconKind {
  const normalized = loader?.replace(/[^a-z]/gi, "").toLowerCase();
  if (normalized === "forge") {
    return "forge";
  }
  if (normalized === "neoforge") {
    return "neoforge";
  }
  if (normalized === "fabric") {
    return "fabric";
  }
  if (normalized === "quilt") {
    return "quilt";
  }
  return "generic";
}

export function getNextOpenModpackMenuId(
  currentOpenId: number | null,
  targetModpackId: number | null,
  target: ModpackMenuPointerTarget,
): number | null {
  if (target === "menuSurface") {
    return currentOpenId;
  }
  if (target === "outside") {
    return null;
  }
  return currentOpenId === targetModpackId ? null : targetModpackId;
}

export function getCompactLibraryNodeGap(): number {
  return compactLibraryNodeGap;
}

export function getModpackMenuPlacement(
  triggerRect: RectLike,
  viewport: ViewportSize,
): { left: number; top: number } {
  const minLeft = modpackMenuSize.viewportPadding;
  const maxLeft = Math.max(minLeft, viewport.width - modpackMenuSize.width - minLeft);
  const left = Math.min(maxLeft, Math.max(minLeft, Math.round(triggerRect.right - modpackMenuSize.width)));
  const belowTop = Math.round(triggerRect.bottom + modpackMenuSize.viewportPadding);
  const wouldOverflowBottom = belowTop + modpackMenuSize.height > viewport.height - modpackMenuSize.viewportPadding;
  const top = wouldOverflowBottom
    ? Math.max(modpackMenuSize.viewportPadding, Math.round(triggerRect.top - modpackMenuSize.viewportPadding - modpackMenuSize.height))
    : belowTop;

  return { left, top };
}

export function getLibraryDialogContent(kind: LibraryDialogKind): LibraryDialogContent {
  switch (kind) {
    case "createScheme":
      return { titleKey: "library.createSchemeTitle", fieldKey: "library.nameLabel", tone: "form" };
    case "renameScheme":
      return { titleKey: "library.renameSchemeTitle", fieldKey: "library.nameLabel", tone: "form" };
    case "renameModpack":
      return { titleKey: "library.renameModpackTitle", fieldKey: "library.nameLabel", tone: "form" };
    case "infoModpack":
      return {
        titleKey: "library.informationTitle",
        bodyKey: "library.informationDescription",
        tone: "info",
      };
    case "deleteScheme":
      return {
        titleKey: "library.deleteSchemeTitle",
        bodyKey: "library.confirmDeleteScheme",
        tone: "danger",
      };
    case "deleteModpack":
      return {
        titleKey: "library.deleteModpackTitle",
        bodyKey: "library.confirmDeleteModpack",
        tone: "danger",
      };
  }
}

export function getInitialExpandedModpackIds(library: LibraryModpack[]): Set<number> {
  return new Set(library.map((modpack) => modpack.id));
}

export function toggleExpandedModpack(current: Set<number>, modpackId: number): Set<number> {
  const next = new Set(current);
  if (next.has(modpackId)) {
    next.delete(modpackId);
  } else {
    next.add(modpackId);
  }
  return next;
}

export function shouldShowSeedFixtureAction(
  library: LibraryModpack[],
  isDevelopment: boolean,
): boolean {
  return isDevelopment && library.length === 0;
}

export function getActiveLibrarySelection(
  library: LibraryModpack[],
  requested: LibrarySelection | null,
): LibrarySelection | null {
  if (requested && hasScheme(library, requested)) {
    return requested;
  }

  for (const modpack of library) {
    const scheme = modpack.schemes[0];
    if (scheme) {
      return { modpackId: modpack.id, schemeId: scheme.id };
    }
  }

  return null;
}

export function getNextSelectionAfterSchemeDelete(
  library: LibraryModpack[],
  deleted: LibrarySelection,
): LibrarySelection | null {
  const modpack = library.find((item) => item.id === deleted.modpackId);
  if (!modpack) {
    return getActiveLibrarySelection(library, null);
  }

  const nextScheme = modpack.schemes.find((scheme) => scheme.id !== deleted.schemeId);
  if (nextScheme) {
    return { modpackId: modpack.id, schemeId: nextScheme.id };
  }

  return getActiveLibrarySelection(
    library.filter((item) => item.id !== deleted.modpackId),
    null,
  );
}

function hasScheme(library: LibraryModpack[], selection: LibrarySelection): boolean {
  return library.some(
    (modpack) =>
      modpack.id === selection.modpackId &&
      modpack.schemes.some((scheme) => scheme.id === selection.schemeId),
  );
}
