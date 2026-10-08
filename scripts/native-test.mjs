// Launch and restart only our own disposable Windows test instance.
import { spawn } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir, version, release, arch } from "node:os";
import { once } from "node:events";
import path from "node:path";
import net from "node:net";
import { fileURLToPath } from "node:url";

if (process.platform !== "win32")
  throw new Error("Native smoke requires Windows");
const root = fileURLToPath(new URL("../", import.meta.url));
// Refuse to attach to someone else's debugging session.
await new Promise((resolve, reject) => {
  const server = net.createServer();
  server.once("error", reject);
  server.listen(9223, "127.0.0.1", () => server.close(resolve));
});
const directory = await mkdtemp(path.join(tmpdir(), "modelshelf-test-"));
const env = {
  ...process.env,
  MODELSHELF_PROFILE: "test",
  MODELSHELF_TEST_DATA_DIR: directory,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9223",
};
console.log(`Disposable profile and screenshots: ${directory}`);
const stages = [];
function childCommand(executable, args, childEnv) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, {
      cwd: root,
      env: childEnv,
      windowsHide: true,
      stdio: "inherit",
      timeout: 180000,
    });
    child.once("error", reject);
    child.once("exit", (code) =>
      code === 0
        ? resolve()
        : reject(new Error(`${executable} failed: ${code}`)),
    );
  });
}
async function run(args) {
  const app = spawn(path.join(root, "target/debug/modelshelf.exe"), [], {
    cwd: root,
    env,
    windowsHide: true,
    stdio: "inherit",
  });
  const exited = new Promise((resolve) => app.once("exit", resolve));
  const stage = { name: args[0] ?? "download-smoke", status: "failed" };
  stages.push(stage);
  try {
    await once(app, "spawn");
    const childEnv = { ...env, MODELSHELF_TEST_PID: String(app.pid) };
    await childCommand(
      process.execPath,
      ["scripts/native-smoke.mjs", ...args],
      childEnv,
    );
    await childCommand(
      "powershell.exe",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        "scripts/windows-dialog.ps1",
        "-ProcessId",
        String(app.pid),
        "-Mode",
        "close-window",
      ],
      childEnv,
    );
    await new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error("Application did not close within 10 seconds")),
        10000,
      );
      exited.then(() => {
        clearTimeout(timer);
        resolve();
      });
    });
    stage.status = "passed";
    stage.shutdown = "native window close";
  } catch (error) {
    stage.error = String(error);
    throw error;
  } finally {
    if (app.exitCode === null && app.pid) app.kill();
    if (app.pid) await exited;
    await writeFile(
      path.join(directory, "native-test.json"),
      JSON.stringify(
        {
          os: version(),
          release: release(),
          architecture: arch(),
          stages,
        },
        null,
        2,
      ),
    );
  }
}
await run([]);
await run(["--restart-check"]);
await run(["--windows-acceptance"]);
console.log("Isolated native smoke and restart checks passed");
