// Launch and restart only our own disposable Windows test instance.
import { spawn } from "node:child_process";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
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
async function run(args) {
  const app = spawn(path.join(root, "target/debug/modelshelf.exe"), [], {
    cwd: root,
    env,
    windowsHide: true,
    stdio: "inherit",
  });
  const exited = new Promise((resolve) => app.once("exit", resolve));
  try {
    await new Promise((resolve, reject) => {
      app.once("error", reject);
      const smoke = spawn(
        process.execPath,
        ["scripts/native-smoke.mjs", ...args],
        {
          cwd: root,
          env,
          windowsHide: true,
          stdio: "inherit",
        },
      );
      smoke.once("error", reject);
      smoke.once("exit", (code) =>
        code === 0
          ? resolve()
          : reject(new Error(`Native smoke failed: ${code}`)),
      );
    });
  } finally {
    if (app.exitCode === null && app.pid) app.kill();
    if (app.pid) await exited;
  }
}
await run([]);
await run(["--restart-check"]);
console.log("Isolated native smoke and restart checks passed");
