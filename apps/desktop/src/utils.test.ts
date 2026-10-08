import { describe, it, expect } from "vitest";
import { bytes, quantization, selectFolder, settingsSchema } from "./utils";
describe("file sizes", () => {
  it("handles bytes, binary units and invalid values", () => {
    expect(bytes(0)).toBe("0 B");
    expect(bytes(1024)).toBe("1 KiB");
    expect(bytes(1024 ** 3)).toBe("1 GiB");
    expect(bytes(-1)).toBe("—");
    expect(bytes(Infinity)).toBe("—");
  });
});
describe("GGUF labels", () => {
  it("recognizes known and future labels without assuming compatibility", () => {
    expect(quantization("model-Q4_K_M.gguf")).toBe("Q4_K_M");
    expect(quantization("Qwen3-8B_Q4_K_M.gguf")).toBe("Q4_K_M");
    expect(quantization("Qwen3.gguf")).toBeNull();
    expect(quantization("model-IQ3_XXS.gguf")).toBe("IQ3_XXS");
    expect(quantization("model-BF16.gguf")).toBe("BF16");
    expect(quantization("config.json")).toBeNull();
  });
});
describe("directory selection", () => {
  const files = [
    { path: "encoder/a.bin", size: 3, sha256: null },
    { path: "encoder/nested/b.bin", size: 4, sha256: null },
    { path: "encoder2/c.bin", size: 9, sha256: null },
  ];
  it("selects nested files without selecting similarly named directories", () => {
    const result = selectFolder(files, new Set(), "encoder/", true);
    expect([...result]).toEqual(["encoder/a.bin", "encoder/nested/b.bin"]);
  });
  it("clears only the chosen subtree and preserves input state", () => {
    const selected = new Set(files.map((f) => f.path));
    expect([...selectFolder(files, selected, "encoder/", false)]).toEqual([
      "encoder2/c.bin",
    ]);
    expect(selected.size).toBe(3);
  });
});
describe("settings validation", () => {
  const base = {
    theme: "dark",
    language: "en",
    concurrency: 2,
    retries: 3,
    notifications: true,
    default_directory: "D:\\Models",
  };
  it("accepts bounded settings", () =>
    expect(settingsSchema.safeParse(base).success).toBe(true));
  it("rejects unsafe concurrency, fractional retries and unsupported language", () => {
    expect(settingsSchema.safeParse({ ...base, concurrency: 4 }).success).toBe(
      false,
    );
    expect(settingsSchema.safeParse({ ...base, retries: 1.5 }).success).toBe(
      false,
    );
    expect(settingsSchema.safeParse({ ...base, language: "xx" }).success).toBe(
      false,
    );
  });
});
