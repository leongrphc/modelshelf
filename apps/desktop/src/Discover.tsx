import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  ArrowLeft,
  Download,
  Search,
  ExternalLink,
  Lock,
  Heart,
} from "lucide-react";
import { command } from "./api";
import type { Repository, Snapshot } from "./types";
import type { T } from "./i18n";
import { bytes, extension, quantization } from "./utils";
import { FileTree } from "./FileTree";
// Resolves to the payload (true for void success), or undefined on cancel/error.
export type Run = (
  name: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;
export function Discover({
  snapshot,
  t,
  run,
}: {
  snapshot: Snapshot;
  t: T;
  run: Run;
}) {
  const [input, setInput] = useState("");
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState("downloads");
  const [task, setTask] = useState("");
  const [repo, setRepo] = useState("");
  const [revision, setRevision] = useState("main");
  const [selected, setSelected] = useState(new Set<string>());
  const [destination, setDestination] = useState(
    snapshot.settings.default_directory,
  );
  const [filter, setFilter] = useState("");
  const [tab, setTab] = useState("files");
  const results = useQuery({
    queryKey: ["search", query, sort, task],
    queryFn: () =>
      command<Repository[]>("search_models", { query, sort, task }),
    enabled: !repo,
    retry: 1,
  });
  const details = useQuery({
    queryKey: ["repository", repo, revision],
    queryFn: () =>
      command<Repository>("repository_details", { repo, revision }),
    enabled: !!repo,
    retry: 1,
  });
  const open = (id: string) => {
    setRepo(id);
    setRevision("main");
    setSelected(new Set());
  };
  const r = details.data;
  const error = repo ? details.error : results.error;
  return (
    <>
      {repo ? (
        <>
          <button className="text-button" onClick={() => setRepo("")}>
            <ArrowLeft size={16} />
            {t("back")}
          </button>
          {details.isPending && <p>{t("loading")}</p>}
          {r && (
            <>
              <div className="repository-heading">
                <div>
                  <div className="eyebrow">
                    HUGGING FACE / {r.id.split("/")[0]}
                  </div>
                  <h1>{r.id.split("/").pop()}</h1>
                  <p className="muted">
                    {r.task ?? t("unknown")} · {r.license ?? t("unknown")} ·{" "}
                    {r.updated_at?.slice(0, 10) ?? t("unknown")}
                  </p>
                </div>
                <button onClick={() => run("open_repository", { repo: r.id })}>
                  <ExternalLink size={15} />
                  {t("providerPage")}
                </button>
              </div>
              {r.gated && (
                <div className="notice">
                  <Lock size={19} />
                  <div>
                    <strong>{t("gated")}</strong>
                    <p>{t("gatedHelp")}</p>
                  </div>
                </div>
              )}
              <div className="toolbar">
                <button
                  className={tab === "files" ? "selected" : ""}
                  onClick={() => setTab("files")}
                >
                  {t("files")} <span className="badge">{r.files.length}</span>
                </button>
                <button
                  className={tab === "readme" ? "selected" : ""}
                  onClick={() => setTab("readme")}
                >
                  {t("readme")}
                </button>
                <label className="inline-label">
                  {t("revision")}
                  <select
                    value={revision}
                    onChange={(e) => {
                      setRevision(e.target.value);
                      setSelected(new Set());
                    }}
                  >
                    {[...new Set(["main", ...r.revisions])].map((v) => (
                      <option key={v}>{v}</option>
                    ))}
                  </select>
                </label>
              </div>
              {tab === "readme" ? (
                <section className="panel">
                  <p className="muted">{t("readmeSafe")}</p>
                  <pre className="readme">{r.readme || t("empty")}</pre>
                </section>
              ) : (
                <>
                  <div className="split">
                    <section className="panel browser">
                      <div className="toolbar">
                        <select
                          aria-label={t("format")}
                          value={filter}
                          onChange={(e) => setFilter(e.target.value)}
                        >
                          <option value="">{t("all")}</option>
                          {[...new Set(r.files.map((f) => extension(f.path)))]
                            .sort()
                            .map((v) => (
                              <option key={v}>{v}</option>
                            ))}
                        </select>
                        <button
                          onClick={() =>
                            setSelected(
                              new Set(
                                r.files
                                  .filter(
                                    (f) =>
                                      !filter || extension(f.path) === filter,
                                  )
                                  .map((f) => f.path),
                              ),
                            )
                          }
                        >
                          {t("selectAll")}
                        </button>
                        <button onClick={() => setSelected(new Set())}>
                          {t("clear")}
                        </button>
                      </div>
                      <FileTree
                        files={r.files.filter(
                          (f) => !filter || extension(f.path) === filter,
                        )}
                        selected={selected}
                        setSelected={setSelected}
                        t={t}
                      />
                    </section>
                    <aside className="panel download-summary">
                      <div className="eyebrow">{t("selected")}</div>
                      <h2>
                        {selected.size} {t("files")}
                      </h2>
                      <div className="big-value mono">
                        {bytes(
                          r.files
                            .filter((f) => selected.has(f.path))
                            .reduce((a, f) => a + f.size, 0),
                        )}
                      </div>
                      <label>
                        {t("destination")}
                        <input
                          value={destination}
                          readOnly
                          placeholder={t("choose")}
                        />
                      </label>
                      <button
                        onClick={async () => {
                          const p = await run("choose_directory");
                          if (typeof p === "string") setDestination(p);
                        }}
                      >
                        {t("choose")}
                      </button>
                      <button
                        className="primary"
                        disabled={!selected.size || !destination}
                        onClick={() =>
                          run("create_download", {
                            repo: r.id,
                            revision: r.sha,
                            selected: [...selected],
                            destination,
                          })
                        }
                      >
                        <Download size={16} />
                        {t("download")}
                      </button>
                      <p className="muted small">{t("verificationHelp")}</p>
                      {snapshot.models.some((m) => m.repository === r.id) && (
                        <span className="badge">{t("installed")}</span>
                      )}
                    </aside>
                  </div>
                  {r.files.some((f) => extension(f.path) === "GGUF") && (
                    <section className="panel">
                      <h2>GGUF · {t("quantization")}</h2>
                      <p className="muted">{t("noPerformance")}</p>
                      <div className="table-wrap">
                        <table>
                          <thead>
                            <tr>
                              <th>{t("name")}</th>
                              <th>{t("quantization")}</th>
                              <th>{t("storageRequired")}</th>
                            </tr>
                          </thead>
                          <tbody>
                            {r.files
                              .filter((f) => extension(f.path) === "GGUF")
                              .map((f) => (
                                <tr key={f.path}>
                                  <td className="mono">{f.path}</td>
                                  <td>
                                    <span className="badge">
                                      {quantization(f.path) ?? t("unknown")}
                                    </span>
                                  </td>
                                  <td>{bytes(f.size)}</td>
                                </tr>
                              ))}
                          </tbody>
                        </table>
                      </div>
                    </section>
                  )}
                </>
              )}
            </>
          )}
        </>
      ) : (
        <>
          <div className="page-heading">
            <div>
              <div className="eyebrow">HUGGING FACE HUB</div>
              <h1>{t("discover")}</h1>
            </div>
          </div>
          <form
            className="search-bar"
            onSubmit={(e) => {
              e.preventDefault();
              const v = input.trim();
              if (
                v.startsWith("https://huggingface.co/") ||
                /^[\w.-]+\/[\w.-]+$/.test(v)
              )
                open(v);
              else setQuery(v);
            }}
          >
            <Search size={20} />
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder={t("ownerSearch")}
              aria-label={t("search")}
            />
            <button className="primary">{t("search")}</button>
          </form>
          <div className="toolbar">
            <select
              aria-label={t("filter")}
              value={sort}
              onChange={(e) => setSort(e.target.value)}
            >
              <option value="downloads">{t("mostDownloads")}</option>
              <option value="likes">{t("popular")}</option>
              <option value="lastModified">{t("updated")}</option>
            </select>
            <select
              aria-label={t("allTasks")}
              value={task}
              onChange={(e) => setTask(e.target.value)}
            >
              <option value="">{t("allTasks")}</option>
              {(
                [
                  "text-generation",
                  "text-to-image",
                  "automatic-speech-recognition",
                  "feature-extraction",
                  "image-classification",
                  "text-to-speech",
                  "image-text-to-text",
                ] as const
              ).map((v, i) => (
                <option key={v} value={v}>
                  {t(
                    (
                      [
                        "text",
                        "image",
                        "speech",
                        "embedding",
                        "vision",
                        "tts",
                        "multimodal",
                      ] as const
                    )[i],
                  )}
                </option>
              ))}
            </select>
          </div>
          {results.isPending && <p>{t("loading")}</p>}
          <div className="model-grid">
            {results.data?.map((m) => (
              <button
                className="model-card"
                key={m.id}
                onClick={() => open(m.id)}
              >
                <div className="card-format">
                  {m.task ?? t("unknown")}
                  {m.gated && <Lock size={14} />}
                </div>
                <span className="muted small">{m.id.split("/")[0]}</span>
                <h3>{m.id.split("/").pop()}</h3>
                <div className="tag-row">
                  {m.tags.slice(0, 3).map((tag) => (
                    <span className="badge" key={tag}>
                      {tag}
                    </span>
                  ))}
                </div>
                <div className="card-foot">
                  <span>
                    <Download size={13} /> {m.downloads.toLocaleString()}
                  </span>
                  <span>
                    <Heart size={13} /> {m.likes.toLocaleString()}
                  </span>
                  <span>{m.license ?? t("unknown")}</span>
                </div>
              </button>
            ))}
          </div>
          {results.data?.length === 0 && <p>{t("noResults")}</p>}
        </>
      )}
      {error && (
        <div role="alert" className="error">
          {String(error)}
          {repo && (
            <>
              <p>{t("gatedHelp")}</p>
              <button onClick={() => run("open_repository", { repo })}>
                {t("providerPage")}
              </button>
            </>
          )}
          <button
            onClick={() => (repo ? details.refetch() : results.refetch())}
          >
            {t("retry")}
          </button>
        </div>
      )}
    </>
  );
}
