import { create } from "zustand";
export type Page =
  "dashboard" | "discover" | "library" | "downloads" | "storage" | "settings";
export const useNavigation = create<{ page: Page; go: (page: Page) => void }>(
  (set) => ({ page: "dashboard", go: (page) => set({ page }) }),
);
