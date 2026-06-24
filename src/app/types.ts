import type { translate } from "../i18n";
import type { LibraryModpack, LibraryScheme } from "../library";

export type Translator = (key: Parameters<typeof translate>[1]) => string;

export type LibraryDialog =
  | { kind: "createScheme"; modpackId: number; name: string; dimensions: [number, number, number] }
  | { kind: "renameScheme"; scheme: LibraryScheme; name: string }
  | { kind: "renameModpack"; modpack: LibraryModpack; name: string }
  | { kind: "infoModpack"; modpack: LibraryModpack }
  | { kind: "deleteScheme"; scheme: LibraryScheme }
  | { kind: "deleteModpack"; modpack: LibraryModpack };
