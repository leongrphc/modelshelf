// Connect to an explicitly launched local ModelShelf WebView2 test session.
// No mocked IPC: all search/download/library state is produced by the Rust backend.
import { chromium } from "playwright-core";
import { mkdir } from "node:fs/promises";
let browser;
for (let attempt = 0; attempt < 30; attempt++) {
  try {
    browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
    break;
  } catch (error) {
    if (attempt === 29) throw error;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
}
let page;
for (let attempt = 0; attempt < 40; attempt++) {
  page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().includes("tauri.localhost"));
  if (page) break;
  await new Promise((resolve) => setTimeout(resolve, 250));
}
if (!page) throw new Error("ModelShelf native webview not found");
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
await page.getByRole("button", { name: "Dashboard", exact: true }).click();
await page.getByRole("heading", { name: "Dashboard", exact: true }).waitFor();
if (process.argv.includes("--restart-check")) {
  await page
    .getByRole("button", { name: "My Library", exact: true })
    .first()
    .click();
  await page
    .getByRole("heading", { name: "openai-community/gpt2", exact: true })
    .first()
    .waitFor();
  console.log("Native restart retained downloaded model");
  await page
    .getByRole("button", { name: "Downloads", exact: true })
    .first()
    .click();
  await page.locator(".status-completed").first().waitFor();
  await page.getByRole("button", { name: "Storage", exact: true }).click();
  await page
    .getByRole("heading", { name: "Hardware information", exact: true })
    .waitFor();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel(/^Appearance/).selectOption("light");
  await page.getByLabel(/^Language/).selectOption("tr");
  await page.getByRole("button", { name: "Save changes", exact: true }).click();
  await page.getByRole("heading", { name: "Ayarlar", exact: true }).waitFor();
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === "light",
  );
  await page.getByLabel(/^Görünüm/).selectOption("dark");
  await page.getByLabel(/^Dil/).selectOption("en");
  await page
    .getByRole("button", { name: "Değişiklikleri kaydet", exact: true })
    .click();
  await page.getByRole("heading", { name: "Settings", exact: true }).waitFor();
  console.log(
    "Downloads, Storage, Settings, Turkish and light/dark theme passed",
  );
  await browser.close();
  process.exit(0);
}
await page
  .getByRole("button", { name: "Discover", exact: true })
  .first()
  .click();
await page
  .getByPlaceholder("Model name, owner, repository ID or Hugging Face URL")
  .fill("openai-community/gpt2");
await page.getByRole("button", { name: "Search", exact: true }).click();
await page.getByRole("heading", { name: "gpt2", exact: true }).waitFor();
await mkdir("docs/screenshots", { recursive: true });
await page.screenshot({ path: "docs/screenshots/discover.png" });
await page.getByRole("heading", { name: "gpt2", exact: true }).click();
await page.getByText("config.json", { exact: true }).first().waitFor();
await page
  .getByRole("checkbox", { name: "config.json", exact: true })
  .first()
  .check();
await page.screenshot({
  path: "docs/screenshots/repository.png",
  mask: [page.locator(".download-summary input")],
  maskColor: "#30303e",
});
// File selection is exercised through the actual accessible checkbox.
const previousJobs = await page.evaluate(async () =>
  (await window.__TAURI_INTERNALS__.invoke("snapshot")).jobs.map((j) => j.id),
);
await page
  .getByRole("button", { name: "Download selected files", exact: true })
  .click();
await page.waitForFunction(
  async (previous) => {
    const current = await window.__TAURI_INTERNALS__.invoke("snapshot");
    const created = current.jobs.find((j) => !previous.includes(j.id));
    if (created?.status === "Failed") throw new Error(created.error);
    return (
      created?.status === "Completed" &&
      current.models.some((m) => m.id === created.id)
    );
  },
  previousJobs,
  { timeout: 60000 },
);
await page
  .getByRole("button", { name: "My Library", exact: true })
  .first()
  .click();
await page
  .getByRole("heading", { name: "openai-community/gpt2", exact: true })
  .first()
  .click();
await page.getByRole("dialog").waitFor();
await page.getByRole("cell", { name: "config.json", exact: true }).waitFor();
await page
  .getByRole("button", { name: "Check integrity", exact: true })
  .click();
await page.getByRole("cell", { name: "Unverified", exact: true }).waitFor();
await page.keyboard.press("Escape");
await page.getByRole("button", { name: "Dashboard", exact: true }).click();
await page.screenshot({
  path: "docs/screenshots/dashboard.png",
  mask: [page.locator(".stats .mono")],
  maskColor: "#30303e",
});
console.log(
  "Native search, file selection, download, library and integrity check passed; page errors:",
  errors,
);
await browser.close();
if (errors.length) process.exitCode = 1;
