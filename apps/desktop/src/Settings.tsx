import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import type { Snapshot } from "./types";
import type { T } from "./i18n";
import type { Run } from "./Discover";
import { bytes, settingsSchema } from "./utils";
import { command } from "./api";
export function Storage({
  snapshot: s,
  t,
  run,
}: {
  snapshot: Snapshot;
  t: T;
  run: Run;
}) {
  const grouped = Object.fromEntries(
    s.usage.by_format.map((f) => [f.format, f.bytes]),
  );
  const sum = Object.values(grouped).reduce((a, b) => a + b, 0);
  return (
    <>
      <div className="page-heading">
        <h1>{t("storage")}</h1>
        <button className="primary" onClick={() => run("add_location")}>
          {t("addLocation")}
        </button>
      </div>
      <p className="notice">{t("logicalHelp")}</p>
      <div className="stats">
        <div>
          <span>{t("managed")}</span>
          <strong>{bytes(s.usage.managed_bytes)}</strong>
        </div>
        <div>
          <span>{t("external")}</span>
          <strong>{bytes(s.usage.external_bytes)}</strong>
        </div>
        <div>
          <span>{t("temporary")}</span>
          <strong>{bytes(s.temporary_bytes)}</strong>
        </div>
      </div>
      <section className="panel">
        <h2>{t("locations")}</h2>
        {!s.locations.length && <p className="muted">{t("noDisk")}</p>}
        {s.locations.map((l) => {
          const disk = s.disks.find((d) => d.path === l.path);
          return (
            <div className="disk" key={l.path}>
              <div>
                <strong className="mono">{l.path}</strong>
                <span className="badge">{l.kind}</span>
              </div>
              {disk?.error ? (
                <p className="error">{disk.error}</p>
              ) : (
                disk && (
                  <>
                    <progress
                      max={disk.total || 1}
                      value={disk.total - disk.free}
                    />
                    <p className="muted">
                      {bytes(disk.free)} {t("free")} / {bytes(disk.total)}{" "}
                      {t("capacity")}
                    </p>
                  </>
                )
              )}
            </div>
          );
        })}
      </section>
      <section className="panel">
        <h2>{t("breakdown")}</h2>
        {Object.entries(grouped)
          .sort((a, b) => b[1] - a[1])
          .map(([ext, size]) => (
            <div className="format-row" key={ext}>
              <span className="mono">{ext}</span>
              <progress value={size} max={sum || 1} />
              <span>{bytes(size)}</span>
            </div>
          ))}
        {!sum && <p>{t("empty")}</p>}
      </section>
      <section className="panel">
        <h2>{t("hardware")}</h2>
        <dl className="metadata">
          <dt>{t("os")}</dt>
          <dd>{s.hardware.os}</dd>
          <dt>{t("cpu")}</dt>
          <dd>{s.hardware.cpu}</dd>
          <dt>{t("ram")}</dt>
          <dd>{bytes(s.hardware.ram)}</dd>
          <dt>{t("gpu")}</dt>
          <dd>{s.hardware.gpu || t("unknown")}</dd>
        </dl>
        <p className="muted">{t("hardwareHelp")}</p>
      </section>
      <section className="panel">
        <h2>{t("duplicates")}</h2>
        <p className="muted">{t("duplicateHelp")}</p>
        {s.usage.duplicates.length ? (
          s.usage.duplicates.map((d, i) => (
            <div className="disk" key={i}>
              <span className="badge">
                {t(d.kind === "same_physical_file" ? "sameFile" : "sameHash")}
              </span>
              {d.paths.map((p) => (
                <p className="mono" key={p}>
                  {p}
                </p>
              ))}
            </div>
          ))
        ) : (
          <p>{t("empty")}</p>
        )}
      </section>
    </>
  );
}
export function SettingsPage({
  snapshot: s,
  t,
  run,
}: {
  snapshot: Snapshot;
  t: T;
  run: Run;
}) {
  const [settings, setSettings] = useState(s.settings);
  const [token, setToken] = useState("");
  const [error, setError] = useState("");
  const account = useQuery({
    queryKey: ["account"],
    queryFn: () => command<string | null>("account_status"),
    retry: false,
  });
  return (
    <>
      <div className="page-heading">
        <h1>{t("settings")}</h1>
        <button
          className="primary"
          onClick={() => {
            const p = settingsSchema.safeParse(settings);
            if (p.success) {
              setError("");
              void run("save_settings", { settings: p.data });
            } else setError(p.error.message);
          }}
        >
          {t("save")}
        </button>
      </div>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <section className="panel settings-section">
        <h2>{t("general")}</h2>
        <label>
          {t("theme")}
          <select
            aria-label={t("theme")}
            value={settings.theme}
            onChange={(e) =>
              setSettings({
                ...settings,
                theme: e.target.value as typeof settings.theme,
              })
            }
          >
            <option value="dark">{t("dark")}</option>
            <option value="light">{t("light")}</option>
            <option value="system">{t("system")}</option>
          </select>
        </label>
        <label>
          {t("language")}
          <select
            aria-label={t("language")}
            value={settings.language}
            onChange={(e) =>
              setSettings({
                ...settings,
                language: e.target.value as typeof settings.language,
              })
            }
          >
            <option value="en">English</option>
            <option value="tr">Türkçe</option>
          </select>
        </label>
      </section>
      <section className="panel settings-section">
        <h2>{t("downloads")}</h2>
        <label>
          {t("destination")}
          <div className="actions">
            <input readOnly value={settings.default_directory} />
            <button
              onClick={async () => {
                const p = await run("choose_directory");
                if (typeof p === "string")
                  setSettings({ ...settings, default_directory: p });
              }}
            >
              {t("choose")}
            </button>
          </div>
        </label>
        <label>
          {t("concurrency")}
          <input
            type="number"
            min="1"
            max="3"
            value={settings.concurrency}
            onChange={(e) =>
              setSettings({ ...settings, concurrency: Number(e.target.value) })
            }
          />
        </label>
        <label>
          {t("retries")}
          <input
            type="number"
            min="0"
            max="5"
            value={settings.retries}
            onChange={(e) =>
              setSettings({ ...settings, retries: Number(e.target.value) })
            }
          />
        </label>
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={settings.notifications}
            onChange={(e) =>
              setSettings({ ...settings, notifications: e.target.checked })
            }
          />
          {t("notifications")}
        </label>
      </section>
      <section className="panel">
        <h2>{t("account")}</h2>
        <p>{account.data ?? t("anonymous")}</p>
        {account.error && <p className="error">{String(account.error)}</p>}
        <label>
          {t("token")}
          <input
            type="password"
            autoComplete="off"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="hf_…"
          />
        </label>
        <div className="actions">
          <button
            disabled={!token}
            onClick={async () => {
              if (await run("connect_account", { token })) {
                setToken("");
                void account.refetch();
              }
            }}
          >
            {t("connect")}
          </button>
          <button
            onClick={async () => {
              await run("disconnect_account");
              void account.refetch();
            }}
          >
            {t("disconnect")}
          </button>
        </div>
      </section>
      <section className="panel">
        <h2>{t("privacy")}</h2>
        <p className="muted">{t("privacyHelp")}</p>
        <button onClick={() => run("export_diagnostics")}>
          {t("diagnostics")}
        </button>
      </section>
      <section className="panel">
        <h2>{t("about")}</h2>
        <p>{t("aboutHelp")}</p>
      </section>
    </>
  );
}
