import { Download, Pause, Play, RotateCcw, X, ArrowUp } from "lucide-react";
import type { Snapshot } from "./types";
import type { T } from "./i18n";
import { statusLabel } from "./i18n";
import type { Run } from "./Discover";
import { bytes } from "./utils";
export function Downloads({
  snapshot,
  t,
  run,
}: {
  snapshot: Snapshot;
  t: T;
  run: Run;
}) {
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">
            {snapshot.jobs.length} {t("total")}
          </div>
          <h1>{t("downloads")}</h1>
        </div>
      </div>
      <p className="notice">{t("resumeNote")}</p>
      {!snapshot.jobs.length && (
        <div className="empty">
          <Download size={40} />
          <p>{t("noTransfers")}</p>
        </div>
      )}
      {snapshot.jobs.map((j) => {
        const size = j.files.reduce((a, f) => a + f.size, 0);
        const done = j.files.reduce((a, f) => a + f.downloaded, 0);
        const active = [
          "Queued",
          "Preparing",
          "Downloading",
          "Resuming",
          "Pausing",
          "Verifying",
        ].includes(j.status);
        return (
          <section className="panel job" key={j.id}>
            <div className="page-heading">
              <div>
                <h2>{j.repository}</h2>
                <p className="mono muted small">
                  {j.revision.slice(0, 12)} · {j.destination}
                </p>
              </div>
              <span className={"status status-" + j.status.toLowerCase()}>
                {statusLabel(j.status, t)}
              </span>
            </div>
            <progress max={size || 1} value={done} />
            <div className="job-metrics mono">
              <span>
                {bytes(done)} / {bytes(size)}
              </span>
              <span>
                {size
                  ? `${Math.min(100, (done / size) * 100).toFixed(1)}%`
                  : "—"}
              </span>
              <span>{bytes(j.speed)}/s</span>
              {j.speed > 0 && size > done && (
                <span>
                  {t("eta")}: {Math.ceil((size - done) / j.speed)}s
                </span>
              )}
            </div>
            {j.error && (
              <p role="alert" className="error">
                {j.error}
              </p>
            )}
            <div className="actions">
              {["Queued", "Preparing", "Downloading", "Resuming"].includes(
                j.status,
              ) && (
                <button
                  onClick={() =>
                    run("download_action", { id: j.id, action: "pause" })
                  }
                >
                  <Pause size={14} />
                  {t("pause")}
                </button>
              )}
              {j.status === "Paused" && (
                <button
                  onClick={() =>
                    run("download_action", { id: j.id, action: "resume" })
                  }
                >
                  <Play size={14} />
                  {t("resume")}
                </button>
              )}
              {["Failed", "Cancelled"].includes(j.status) && (
                <button
                  onClick={() =>
                    run("download_action", { id: j.id, action: "retry" })
                  }
                >
                  <RotateCcw size={14} />
                  {t("retry")}
                </button>
              )}
              {(active || j.status === "Paused") && (
                <button
                  onClick={() =>
                    run("download_action", { id: j.id, action: "cancel" })
                  }
                >
                  <X size={14} />
                  {t("cancel")}
                </button>
              )}
              {j.status === "Queued" && (
                <button
                  onClick={() =>
                    run("download_action", { id: j.id, action: "up" })
                  }
                >
                  <ArrowUp size={14} />
                  {t("up")}
                </button>
              )}
            </div>
            <details>
              <summary>
                {j.files.length} {t("files")}
              </summary>
              {j.files.map((f) => (
                <div className="transfer-file" key={f.path}>
                  <span className="mono">{f.path}</span>
                  <span>
                    {statusLabel(f.status, t)} ·{" "}
                    {statusLabel(f.verification, t)}
                  </span>
                  <progress max={f.size || 1} value={f.downloaded} />
                  <span className="muted small">
                    {bytes(f.downloaded)} / {bytes(f.size)} · {t("reused")}:{" "}
                    {bytes(f.reused_bytes)}
                  </span>
                </div>
              ))}
            </details>
          </section>
        );
      })}
    </>
  );
}
