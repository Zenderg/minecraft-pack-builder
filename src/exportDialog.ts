import { save } from "@tauri-apps/plugin-dialog";

export type ExportFormat = "schem" | "litematic";

export async function chooseExportDestination({
  defaultFileName,
  format,
}: {
  defaultFileName: string;
  format: ExportFormat;
}): Promise<string | null> {
  const selected = await save({
    defaultPath: defaultFileName,
    filters: [
      {
        name: format === "schem" ? "Sponge schematic" : "Litematica schematic",
        extensions: [format],
      },
    ],
  });

  return selected;
}
