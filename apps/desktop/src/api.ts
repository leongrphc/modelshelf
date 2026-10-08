import { invoke, isTauri } from "@tauri-apps/api/core";
export const desktop = isTauri();
export async function command<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!desktop)
    throw new Error(
      "Open ModelShelf as a desktop application to access your library. Run pnpm tauri dev.",
    );
  return invoke<T>(name, args);
}
