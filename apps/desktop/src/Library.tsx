import { useState, useEffect, useRef } from "react";
import {
  FolderOpen,
  Star,
  FileBox,
  Grid2X2,
  List,
  Search,
  X,
} from "lucide-react";
import type { Model, Snapshot } from "./types";
import type { T } from "./i18n";
import { statusLabel } from "./i18n";
import type { Run } from "./Discover";
import { bytes, extension, quantization } from "./utils";
export function ImportButtons({ t, run }: { t: T; run: Run }) {
  return (
    <div className="actions">
      <button onClick={() => run("import_model", { directory: false })}>
        <FileBox size={15} />
        {t("importFile")}
      </button>
      <button onClick={() => run("import_model", { directory: true })}>
        <FolderOpen size={15} />
        {t("importFolder")}
      </button>
    </div>
  );
}
export function Library({
  snapshot,
  t,
  run,
}: {
  snapshot: Snapshot;
  t: T;
  run: Run;
}) {
  const [search, setSearch] = useState("");
  const [format, setFormat] = useState("");
  const [folder, setFolder] = useState("");
  const [sort, setSort] = useState("added");
  const [grid, setGrid] = useState(true);
  const [detail, setDetail] = useState<string | null>(null);
  const models = snapshot.models
    .filter(
      (m) =>
        (m.display_name + " " + m.tags.join(" "))
          .toLowerCase()
          .includes(search.toLowerCase()) &&
        (!format || m.files.some((f) => extension(f.path) === format)) &&
        (!folder || m.path.startsWith(folder)),
    )
    .sort((a, b) =>
      sort === "name"
        ? a.display_name.localeCompare(b.display_name)
        : sort === "size"
          ? total(b) - total(a)
          : b.created_at.localeCompare(a.created_at),
    );
  const m = snapshot.models.find((m) => m.id === detail);
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">
            {snapshot.models.length} {t("models")}
          </div>
          <h1>{t("library")}</h1>
        </div>
        <ImportButtons t={t} run={run} />
      </div>
      <div className="toolbar">
        <div className="input-icon">
          <Search size={16} />
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t("searchLibrary")}
          />
        </div>
        <select
          aria-label={t("format")}
          value={format}
          onChange={(e) => setFormat(e.target.value)}
        >
          <option value="">{t("all")}</option>
          {[
            ...new Set(
              snapshot.models.flatMap((m) =>
                m.files.map((f) => extension(f.path)),
              ),
            ),
          ].map((f) => (
            <option key={f}>{f}</option>
          ))}
        </select>
        <select
          aria-label={t("locations")}
          value={folder}
          onChange={(e) => setFolder(e.target.value)}
        >
          <option value="">{t("locationFilter")}</option>
          {snapshot.locations.map((l) => (
            <option key={l.path}>{l.path}</option>
          ))}
        </select>
        <select
          aria-label={t("filter")}
          value={sort}
          onChange={(e) => setSort(e.target.value)}
        >
          <option value="added">{t("added")}</option>
          <option value="name">{t("name")}</option>
          <option value="size">{t("size")}</option>
        </select>
        <button
          aria-label={grid ? t("list") : t("grid")}
          onClick={() => setGrid(!grid)}
        >
          {grid ? <List size={18} /> : <Grid2X2 size={18} />}
        </button>
      </div>
      {models.length ? (
        <div className={grid ? "model-grid" : "model-list"}>
          {models.map((model) => (
            <article className="model-card" key={model.id}>
              <div className="card-format">
                {model.ownership === "managed" ? t("managed") : t("external")}
                <button
                  className={
                    "icon-button " + (model.favorite ? "favorited" : "")
                  }
                  aria-label={t("favorite")}
                  aria-pressed={model.favorite}
                  onClick={() =>
                    run("update_model", {
                      id: model.id,
                      favorite: !model.favorite,
                      notes: model.notes,
                      tags: model.tags,
                    })
                  }
                >
                  <Star
                    size={16}
                    fill={model.favorite ? "currentColor" : "none"}
                  />
                </button>
              </div>
              <button
                className="model-title"
                onClick={() => setDetail(model.id)}
              >
                <h3>{model.display_name}</h3>
              </button>
              <p className="mono muted truncate">{model.path}</p>
              <div className="tag-row">
                <span className="badge">{bytes(total(model))}</span>
                {model.tags.map((tag) => (
                  <span className="badge" key={tag}>
                    {tag}
                  </span>
                ))}
              </div>
              <div className="card-foot">
                <span>
                  {model.files.length} {t("files")}
                </span>
                <span>{model.created_at.slice(0, 10)}</span>
                <button
                  className="text-button"
                  onClick={() => setDetail(model.id)}
                >
                  {t("details")}
                </button>
              </div>
            </article>
          ))}
        </div>
      ) : (
        <div className="empty">
          <FileBox size={40} />
          <h2>{t("emptyLibrary")}</h2>
          <p>{t("emptyLibraryHelp")}</p>
        </div>
      )}
      {m && (
        <ModelDetail
          key={m.id}
          model={m}
          t={t}
          run={run}
          close={() => setDetail(null)}
        />
      )}
    </>
  );
}
export function total(m: Model) {
  return m.files.reduce((a, f) => a + f.size, 0);
}
function ModelDetail({
  model: m,
  t,
  run,
  close,
}: {
  model: Model;
  t: T;
  run: Run;
  close: () => void;
}) {
  const [notes, setNotes] = useState(m.notes);
  const [tags, setTags] = useState(m.tags.join(", "));
  const dialog = useRef<HTMLElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    dialog.current?.querySelector<HTMLElement>("button")?.focus();
    return () => previous?.focus();
  }, []);
  return (
    <div
      className="modal-scrim"
      onClick={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <section
        ref={dialog}
        role="dialog"
        aria-modal="true"
        aria-label={m.display_name}
        className="modal"
        onKeyDown={(e) => {
          if (e.key === "Escape") close();
          if (e.key === "Tab") {
            const nodes = dialog.current?.querySelectorAll<HTMLElement>(
              'button:not(:disabled),input,textarea,select,[tabindex="0"]',
            );
            if (nodes?.length) {
              const first = nodes[0],
                last = nodes[nodes.length - 1];
              if (e.shiftKey && document.activeElement === first) {
                e.preventDefault();
                last.focus();
              } else if (!e.shiftKey && document.activeElement === last) {
                e.preventDefault();
                first.focus();
              }
            }
          }
        }}
      >
        <div className="page-heading">
          <h2>{m.display_name}</h2>
          <button aria-label={t("close")} onClick={close} autoFocus>
            <X size={18} />
          </button>
        </div>
        <p className="notice">
          {t(m.ownership === "external" ? "externalHelp" : "managedHelp")}
        </p>
        <dl className="metadata">
          <dt>{t("path")}</dt>
          <dd className="mono">{m.path}</dd>
          <dt>{t("revision")}</dt>
          <dd className="mono">{m.revision ?? t("unknown")}</dd>
          <dt>{t("repository")}</dt>
          <dd>{m.repository ?? t("unknown")}</dd>
        </dl>
        <div className="actions">
          <button onClick={() => run("open_model_folder", { id: m.id })}>
            {t("openFolder")}
          </button>
          <button
            onClick={() =>
              navigator.clipboard
                .writeText(m.path)
                .catch((e) => run("clipboard_error", { error: String(e) }))
            }
          >
            {t("copyPath")}
          </button>
          <button onClick={() => run("verify_model", { id: m.id })}>
            {t("verify")}
          </button>
          <button onClick={() => run("recheck_model", { id: m.id })}>
            {t("recheck")}
          </button>
          <button onClick={() => run("rescan_model", { id: m.id })}>
            {t("rescan")}
          </button>
        </div>
        <p className="muted small">{t("verificationHelp")}</p>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t("files")}</th>
                <th>{t("size")}</th>
                <th>{t("verification")}</th>
              </tr>
            </thead>
            <tbody>
              {m.files.map((f) => (
                <tr key={f.path}>
                  <td className="mono">
                    {f.relative_path}
                    {quantization(f.path) && (
                      <span className="badge">{quantization(f.path)}</span>
                    )}
                  </td>
                  <td>{bytes(f.size)}</td>
                  <td>{statusLabel(f.verification, t)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <label>
          {t("notes")}
          <textarea value={notes} onChange={(e) => setNotes(e.target.value)} />
        </label>
        <label>
          {t("tags")}
          <input value={tags} onChange={(e) => setTags(e.target.value)} />
        </label>
        <div className="actions">
          <button
            className="primary"
            onClick={() =>
              run("update_model", {
                id: m.id,
                favorite: m.favorite,
                notes,
                tags: tags
                  .split(",")
                  .map((t) => t.trim())
                  .filter(Boolean),
              })
            }
          >
            {t("save")}
          </button>
          <button
            onClick={async () => {
              await run("remove_model", { id: m.id });
              close();
            }}
          >
            {t("remove")}
          </button>
          {m.ownership === "managed" && (
            <button
              className="danger"
              onClick={async () => {
                await run("delete_model", { id: m.id });
                close();
              }}
            >
              {t("delete")}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}
