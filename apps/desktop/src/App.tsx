import { useState, useEffect } from "react";
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import {
  LibraryBig,
  LayoutDashboard,
  Compass,
  Download,
  HardDrive,
  Settings as SettingsIcon,
  ArrowUpRight,
  Search,
  AlertCircle,
  RefreshCw,
  Star,
} from "lucide-react";
import { command, desktop } from "./api";
import type { Snapshot } from "./types";
import { translator, type T } from "./i18n";
import { useNavigation, type Page } from "./store";
import { bytes } from "./utils";
import { Discover, type Run } from "./Discover";
import { Library, ImportButtons, total } from "./Library";
import { Downloads } from "./Downloads";
import { SettingsPage, Storage } from "./Settings";
const icons = {
  dashboard: LayoutDashboard,
  discover: Compass,
  library: LibraryBig,
  downloads: Download,
  storage: HardDrive,
  settings: SettingsIcon,
};
const client = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
});
export default function App() {
  return (
    <QueryClientProvider client={client}>
      <Workspace />
    </QueryClientProvider>
  );
}
function Workspace() {
  const { page, go } = useNavigation();
  const [message, setMessage] = useState<{
    text: string;
    error: boolean;
  } | null>(null);
  const [busy, setBusy] = useState(0);
  const cache = useQueryClient();
  const snapshot = useQuery({
    queryKey: ["snapshot"],
    queryFn: () => command<Snapshot>("snapshot"),
    enabled: desktop,
    refetchInterval: 15000,
  });
  const s = snapshot.data;
  const t = translator(s?.settings.language ?? "en");
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    void listen("state-changed", () => {
      void cache.invalidateQueries({ queryKey: ["snapshot"] });
    }).then((fn) => {
      if (disposed) fn();
      else cleanup = fn;
    });
    return () => {
      disposed = true;
      cleanup?.();
    };
  }, [cache]);
  useEffect(() => {
    const theme = s?.settings.theme ?? "dark";
    const media = matchMedia("(prefers-color-scheme: dark)");
    const update = () => {
      document.documentElement.dataset.theme =
        theme === "system" ? (media.matches ? "dark" : "light") : theme;
    };
    update();
    media.addEventListener("change", update);
    document.documentElement.lang = s?.settings.language ?? "en";
    return () => media.removeEventListener("change", update);
  }, [s?.settings.theme, s?.settings.language]);
  useEffect(() => {
    if (!message) return;
    const timer = setTimeout(() => setMessage(null), 7000);
    return () => clearTimeout(timer);
  }, [message]);
  const run: Run = async (name, args) => {
    setBusy((n) => n + 1);
    try {
      if (name === "clipboard_error") throw new Error(String(args?.error));
      const result = await command(name, args);
      await cache.invalidateQueries({ queryKey: ["snapshot"] });
      if (name === "create_download") go("downloads");
      if (!["choose_directory", "import_model", "add_location"].includes(name))
        setMessage({ text: t("success"), error: false });
      return result;
    } catch (e) {
      setMessage({ text: String(e), error: true });
      return undefined;
    } finally {
      setBusy((n) => n - 1);
    }
  };
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <LibraryBig size={24} />
          </div>
          <span>
            ModelShelf<span className="version">0.1</span>
          </span>
        </div>
        <div className="sidebar-label">{t("localFirst")}</div>
        <nav aria-label="Main">
          {(Object.keys(icons) as Page[]).map((p) => {
            const Icon = icons[p];
            return (
              <button
                key={p}
                className={page === p ? "nav-item active" : "nav-item"}
                aria-current={page === p ? "page" : undefined}
                onClick={() => go(p)}
              >
                <Icon size={19} />
                <span>{t(p)}</span>
                {p === "downloads" &&
                  !!s?.jobs.filter((j) =>
                    [
                      "Downloading",
                      "Queued",
                      "Preparing",
                      "Resuming",
                      "Verifying",
                    ].includes(j.status),
                  ).length && (
                    <span className="nav-count">
                      {
                        s.jobs.filter((j) =>
                          [
                            "Downloading",
                            "Queued",
                            "Preparing",
                            "Resuming",
                            "Verifying",
                          ].includes(j.status),
                        ).length
                      }
                    </span>
                  )}
              </button>
            );
          })}
        </nav>
        <div className="sidebar-bottom">
          <span className="local-dot" />
          {t("tagline")}
          <div className="mono">LOCAL · OPEN SOURCE</div>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <span>
            ModelShelf <span className="slash">/</span> {t(page)}
          </span>
          <button
            className="icon-button"
            aria-label={t("refresh")}
            onClick={() => snapshot.refetch()}
          >
            <RefreshCw
              size={16}
              className={snapshot.isFetching ? "spin" : ""}
            />
          </button>
        </header>
        <div className="content" aria-busy={busy > 0}>
          {!desktop ? (
            <div className="empty">
              <LibraryBig size={48} />
              <h1>{t("desktopRequired")}</h1>
              <p>{t("desktopHelp")}</p>
            </div>
          ) : snapshot.isPending ? (
            <div className="empty">{t("loading")}</div>
          ) : snapshot.error ? (
            <div className="error" role="alert">
              <AlertCircle />
              {String(snapshot.error)}
              <button onClick={() => snapshot.refetch()}>{t("retry")}</button>
            </div>
          ) : (
            s && (
              <>
                {page === "dashboard" && (
                  <Dashboard snapshot={s} t={t} run={run} />
                )}
                {page === "discover" && (
                  <Discover snapshot={s} t={t} run={run} />
                )}
                {page === "library" && <Library snapshot={s} t={t} run={run} />}
                {page === "downloads" && (
                  <Downloads snapshot={s} t={t} run={run} />
                )}
                {page === "storage" && <Storage snapshot={s} t={t} run={run} />}
                {page === "settings" && (
                  <SettingsPage snapshot={s} t={t} run={run} />
                )}
              </>
            )
          )}
        </div>
      </main>
      {message && (
        <div
          className={"toast " + (message.error ? "error" : "")}
          role={message.error ? "alert" : "status"}
        >
          <span>{message.text}</span>
          <button aria-label={t("close")} onClick={() => setMessage(null)}>
            ×
          </button>
        </div>
      )}
    </div>
  );
}
function Dashboard({
  snapshot: s,
  t,
  run,
}: {
  snapshot: Snapshot;
  t: T;
  run: Run;
}) {
  const { go } = useNavigation();
  const active = s.jobs.filter((j) =>
    ["Queued", "Preparing", "Downloading", "Resuming", "Verifying"].includes(
      j.status,
    ),
  );
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">{t("localFirst")}</div>
          <h1>{t("dashboard")}</h1>
          <p className="muted">{t("tagline")}</p>
        </div>
        <ImportButtons t={t} run={run} />
      </div>
      <div className="stats">
        <div>
          <span>{t("models")}</span>
          <strong>
            {s.models.length}
            <small>
              {s.models.filter((m) => m.ownership === "managed").length}{" "}
              {t("managed")}
            </small>
          </strong>
        </div>
        <div>
          <span>{t("size")}</span>
          <strong>{bytes(s.usage.unique_logical_bytes)}</strong>
        </div>
        <div>
          <span>{t("free")}</span>
          <strong>
            {s.disks[0] && !s.disks[0].error ? bytes(s.disks[0].free) : "—"}
          </strong>
          {s.disks[0] && (
            <span className="small mono truncate">{s.disks[0].path}</span>
          )}
        </div>
        <div>
          <span>{t("active")}</span>
          <strong>{active.length}</strong>
        </div>
      </div>
      <button className="dashboard-search" onClick={() => go("discover")}>
        <Search size={19} />
        <span>{t("ownerSearch")}</span>
        <ArrowUpRight size={18} />
      </button>
      <div className="section-heading">
        <h2>{t("recent")}</h2>
        <button className="text-button" onClick={() => go("library")}>
          {t("library")}
          <ArrowUpRight size={15} />
        </button>
      </div>
      {s.models.length ? (
        <div className="model-grid">
          {[...s.models]
            .sort((a, b) => b.created_at.localeCompare(a.created_at))
            .slice(0, 6)
            .map((m) => (
              <button
                className="model-card"
                key={m.id}
                onClick={() => go("library")}
              >
                <div className="card-format">
                  {t(m.ownership === "managed" ? "managed" : "external")}
                  {m.favorite && <Star size={14} />}
                </div>
                <h3>{m.display_name}</h3>
                <p className="mono muted truncate">{m.repository ?? m.path}</p>
                <div className="card-foot">
                  <span>{bytes(total(m))}</span>
                  <span>
                    {m.files.length} {t("files")}
                  </span>
                </div>
              </button>
            ))}
        </div>
      ) : (
        <div className="empty panel">
          <LibraryBig size={38} />
          <h2>{t("emptyLibrary")}</h2>
          <p>{t("emptyLibraryHelp")}</p>
          <button className="primary" onClick={() => go("discover")}>
            {t("discover")}
            <ArrowUpRight size={16} />
          </button>
        </div>
      )}
      <div className="two-columns">
        <section className="panel">
          <h2>{t("favorites")}</h2>
          {s.models
            .filter((m) => m.favorite)
            .slice(0, 5)
            .map((m) => (
              <button
                className="summary-row"
                key={m.id}
                onClick={() => go("library")}
              >
                <Star size={15} />
                <span>{m.display_name}</span>
                <span className="muted mono">{bytes(total(m))}</span>
              </button>
            ))}
          {!s.models.some((m) => m.favorite) && (
            <p className="muted">{t("empty")}</p>
          )}
        </section>
        <section className="panel">
          <h2>{t("completed")}</h2>
          {s.jobs
            .filter((j) => j.status === "Completed")
            .slice(0, 5)
            .map((j) => (
              <button
                className="summary-row"
                key={j.id}
                onClick={() => go("downloads")}
              >
                <Download size={15} />
                <span>{j.repository}</span>
              </button>
            ))}
          {!s.jobs.some((j) => j.status === "Completed") && (
            <p className="muted">{t("empty")}</p>
          )}
          <button className="text-button" onClick={() => go("downloads")}>
            {t("openDownloads")}
            <ArrowUpRight size={14} />
          </button>
        </section>
      </div>
    </>
  );
}
