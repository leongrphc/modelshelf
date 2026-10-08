import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile, realpath, rename } from "node:fs/promises";
import path from "node:path";
import os from "node:os";

export async function windowsAcceptance(page) {
  const root = process.env.MODELSHELF_TEST_DATA_DIR;
  const results = [];
  async function scenario(name, expected, operation) {
    try {
      await operation();
      results.push({ name, expected, observed: expected, status: "passed" });
      console.log(`Windows acceptance: ${name} passed`);
    } catch (error) {
      results.push({
        name,
        expected,
        observed: String(error),
        status: "failed",
      });
      throw error;
    } finally {
      await writeFile(
        path.join(root, "windows-acceptance.json"),
        JSON.stringify(
          {
            os: os.version(),
            release: os.release(),
            architecture: os.arch(),
            date: new Date().toISOString(),
            results,
          },
          null,
          2,
        ),
      );
    }
  }
  function dialog(mode, fixture) {
    return new Promise((resolve, reject) => {
      const child = spawn(
        "powershell.exe",
        [
          "-NoProfile",
          "-ExecutionPolicy",
          "Bypass",
          "-File",
          "scripts/windows-dialog.ps1",
          "-ProcessId",
          process.env.MODELSHELF_TEST_PID,
          "-Mode",
          mode,
          ...(fixture ? ["-Path", fixture] : []),
        ],
        { windowsHide: true, stdio: "inherit", timeout: 20000 },
      );
      child.once("error", reject);
      child.once("exit", (code) =>
        code === 0
          ? resolve()
          : reject(new Error(`Native dialog ${mode} failed: ${code}`)),
      );
    });
  }
  const invoke = (command, args) =>
    page.evaluate(
      ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
      { command, args },
    );
  const snapshot = () => invoke("snapshot");
  async function settledWithoutNotification() {
    await page.locator('.content[aria-busy="false"]').waitFor();
    assert.equal(await page.locator(".toast").count(), 0);
  }
  async function waitSnapshot(predicate) {
    const deadline = Date.now() + 30000;
    do {
      const state = await snapshot();
      const result = predicate(state);
      if (result) return result;
      await new Promise((resolve) => setTimeout(resolve, 100));
    } while (Date.now() < deadline);
    throw new Error(
      "Expected native application state was not reached within 30 seconds",
    );
  }
  async function waitModel(name) {
    return waitSnapshot((state) =>
      state.models.find((m) => m.display_name === name),
    );
  }
  const fixtures = path.join(root, "Türkçe boşluklu modeller");
  await mkdir(fixtures);
  const source = path.join(fixtures, "örnek ağırlık.gguf");
  const original = Buffer.from("ModelShelf disposable fixture — özgün içerik");
  await writeFile(source, original);
  await page
    .getByRole("button", { name: "My Library", exact: true })
    .first()
    .click();
  let external;
  await scenario(
    "File picker import",
    "External file indexed with its original bytes unchanged",
    async () => {
      await page
        .getByRole("button", { name: "Import file", exact: true })
        .click();
      await dialog("choose-file", source);
      external = await waitModel(path.basename(source));
      assert.equal(external.ownership, "external");
      assert.deepEqual(await readFile(source), original);
    },
  );
  await scenario(
    "External deletion denied",
    "Backend rejects permanent deletion of external model; bytes unchanged",
    async () => {
      await assert.rejects(
        invoke("delete_model", { id: external.id }),
        /External files cannot be deleted/,
      );
      assert.deepEqual(await readFile(source), original);
    },
  );
  await scenario(
    "Remove external library entry",
    "Entry removed while source file remains unchanged",
    async () => {
      await page
        .getByRole("button", { name: path.basename(source), exact: true })
        .click();
      await page
        .getByRole("button", { name: "Remove from library", exact: true })
        .click();
      await waitSnapshot(
        (state) => !state.models.some((m) => m.id === external.id),
      );
      assert.deepEqual(await readFile(source), original);
    },
  );
  await scenario(
    "Folder picker and index confirmation",
    "Native confirmation accepted; folder indexed without changing files",
    async () => {
      await page
        .getByRole("button", { name: "Index folder", exact: true })
        .click();
      await dialog("choose-folder", fixtures);
      await dialog("yes");
      const model = await waitModel(path.basename(fixtures));
      assert.equal(model.ownership, "external");
      assert.equal(model.files.length, 1);
      assert.deepEqual(await readFile(source), original);
    },
  );
  await scenario(
    "Storage folder picker",
    "Selected Turkish/space path registered as managed storage",
    async () => {
      const operation = invoke("choose_directory");
      await dialog("choose-folder", fixtures);
      const chosen = await operation;
      assert.equal(await realpath(chosen), await realpath(fixtures));
      assert(
        (await snapshot()).locations.some(
          (l) => l.path === chosen && l.kind === "managed",
        ),
      );
    },
  );
  await scenario(
    "Rescan declined feedback",
    "No keeps detail open with no success or error notice",
    async () => {
      await page
        .getByRole("button", { name: path.basename(fixtures), exact: true })
        .click();
      const before = (await snapshot()).models;
      await page
        .getByRole("button", { name: "Rescan files", exact: true })
        .click();
      await dialog("no");
      await settledWithoutNotification();
      assert.equal(await page.getByRole("dialog").count(), 1);
      assert.deepEqual((await snapshot()).models, before);
      await page.keyboard.press("Escape");
    },
  );
  await scenario(
    "Rescan confirmed feedback",
    "Confirmed rescan refreshes files and reports success",
    async () => {
      await page
        .getByRole("button", { name: path.basename(fixtures), exact: true })
        .click();
      await page
        .getByRole("button", { name: "Rescan files", exact: true })
        .click();
      await dialog("yes");
      await page.locator('.toast[role="status"]').waitFor();
      assert.equal((await waitModel(path.basename(fixtures))).files.length, 1);
      await page.keyboard.press("Escape");
    },
  );
  await scenario(
    "Failed account input retained",
    "Invalid input shows error while preserving the entered token",
    async () => {
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      const token = page.locator('input[type="password"]');
      await token.fill("invalid-test-token");
      await page
        .getByRole("button", { name: "Connect account", exact: true })
        .click();
      await page.locator('.toast[role="alert"]').waitFor();
      assert.equal(await token.inputValue(), "invalid-test-token");
      assert.equal(await page.locator('.toast[role="status"]').count(), 0);
    },
  );
  await scenario(
    "Diagnostics save cancelled feedback",
    "Cancelling clears stale notice and shows no success or error",
    async () => {
      await page
        .getByRole("button", { name: "Export diagnostics", exact: true })
        .click();
      await dialog("cancel");
      await settledWithoutNotification();
      await page
        .getByRole("button", { name: "My Library", exact: true })
        .first()
        .click();
    },
  );
  await scenario(
    "File picker cancellation",
    "Cancelled picker does not change library",
    async () => {
      const before = (await snapshot()).models.map((m) => m.id).sort();
      const operation = invoke("import_model", { directory: false });
      await dialog("cancel");
      assert.equal(await operation, null);
      assert.deepEqual(
        (await snapshot()).models.map((m) => m.id).sort(),
        before,
      );
    },
  );
  const managed = (await snapshot()).models.find(
    (m) => m.ownership === "managed",
  );
  assert(managed, "Smoke download must precede deletion acceptance");
  // Never delete anything unless the backend's recorded file is inside this fixture.
  const dataRoot = await realpath(root);
  for (const file of managed.files) {
    const relative = path.relative(dataRoot, await realpath(file.path));
    assert(
      relative && !relative.startsWith("..") && !path.isAbsolute(relative),
    );
  }
  const keep = path.join(managed.path, "unrelated-keep.txt");
  await writeFile(keep, "preserve me");
  await scenario(
    "Permanent deletion declined",
    "Native No preserves managed files, library entry and detail without success notice",
    async () => {
      const before = await readFile(managed.files[0].path);
      await page
        .getByRole("button", { name: managed.display_name, exact: true })
        .click();
      await page
        .getByRole("button", { name: "Delete owned files", exact: true })
        .click();
      await dialog("no");
      await settledWithoutNotification();
      assert.equal(await page.getByRole("dialog").count(), 1);
      await page.screenshot({
        path: path.join(root, "screenshots", "delete-cancelled.png"),
      });
      assert.deepEqual(await readFile(managed.files[0].path), before);
      assert((await snapshot()).models.some((m) => m.id === managed.id));
    },
  );
  await scenario(
    "Failed deletion feedback",
    "Unavailable directory shows an error, retains detail and permits retry",
    async () => {
      const moved = managed.path + "-unavailable";
      await rename(managed.path, moved);
      try {
        await page
          .getByRole("button", { name: "Delete owned files", exact: true })
          .click();
        await dialog("yes");
        await page.locator('.toast[role="alert"]').waitFor();
        await page.locator('.content[aria-busy="false"]').waitFor();
        assert.equal(await page.getByRole("dialog").count(), 1);
        assert.equal(await page.locator('.toast[role="status"]').count(), 0);
        assert((await snapshot()).models.some((m) => m.id === managed.id));
      } finally {
        await rename(moved, managed.path);
      }
    },
  );
  await scenario(
    "Permanent deletion confirmed",
    "Native Yes deletes recorded files only, preserving unrelated file",
    async () => {
      await page
        .getByRole("button", { name: "Delete owned files", exact: true })
        .click();
      await dialog("yes");
      await page.locator('.toast[role="status"]').waitFor();
      await page.getByRole("dialog").waitFor({ state: "detached" });
      for (const file of managed.files)
        await assert.rejects(readFile(file.path), { code: "ENOENT" });
      assert.equal(await readFile(keep, "utf8"), "preserve me");
      assert(!(await snapshot()).models.some((m) => m.id === managed.id));
      assert.deepEqual(await readFile(source), original);
    },
  );
}
