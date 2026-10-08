import { z } from "zod";
import type { RemoteFile } from "./types";
export function bytes(value: number) {
  if (!Number.isFinite(value) || value < 0) return "—";
  if (value === 0) return "0 B";
  const i = Math.min(Math.floor(Math.log(value) / Math.log(1024)), 4);
  return `${(value / 1024 ** i).toLocaleString(undefined, { maximumFractionDigits: i ? 1 : 0 })} ${["B", "KiB", "MiB", "GiB", "TiB"][i]}`;
}
export function quantization(path: string) {
  return (
    (path.split(/[\\/]/).pop() ?? path)
      .match(/(?:^|[-_.])(IQ\d[\w]*|Q\d[\w]*|BF16|F16|F32)(?=[-.]|$)/i)?.[1]
      .toUpperCase() ?? null
  );
}
export function extension(path: string) {
  return path.split(".").pop()?.toUpperCase() ?? "";
}
export function selectFolder(
  files: RemoteFile[],
  selected: Set<string>,
  prefix: string,
  checked: boolean,
) {
  const next = new Set(selected);
  files
    .filter((f) => f.path.startsWith(prefix))
    .forEach((f) => (checked ? next.add(f.path) : next.delete(f.path)));
  return next;
}
export const settingsSchema = z.object({
  theme: z.enum(["dark", "light", "system"]),
  language: z.enum(["en", "tr"]),
  concurrency: z.number().int().min(1).max(3),
  retries: z.number().int().min(0).max(5),
  notifications: z.boolean(),
  default_directory: z.string(),
});
