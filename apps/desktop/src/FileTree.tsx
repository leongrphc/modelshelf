import { useState } from "react";
import { Folder, File, ChevronDown, ChevronRight } from "lucide-react";
import type { RemoteFile } from "./types";
import type { T } from "./i18n";
import { bytes, selectFolder } from "./utils";
export function FileTree({
  files,
  selected,
  setSelected,
  t,
  prefix = "",
}: {
  files: RemoteFile[];
  selected: Set<string>;
  setSelected: (s: Set<string>) => void;
  t: T;
  prefix?: string;
}) {
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const children = new Set(
    files
      .filter((f) => f.path.startsWith(prefix))
      .map((f) => f.path.slice(prefix.length).split("/")[0]),
  );
  return (
    <div className="file-tree">
      {[...children].sort().map((name) => {
        const path = prefix + name;
        const file = files.find((f) => f.path === path);
        const nested = files.filter((f) => f.path.startsWith(path + "/"));
        const checked = file
          ? selected.has(path)
          : nested.every((f) => selected.has(f.path));
        return (
          <div key={path}>
            <div className="file-row">
              <input
                type="checkbox"
                aria-label={path}
                checked={checked}
                onChange={(e) =>
                  setSelected(
                    file
                      ? selectFolder([file], selected, path, e.target.checked)
                      : selectFolder(
                          files,
                          selected,
                          path + "/",
                          e.target.checked,
                        ),
                  )
                }
              />
              {!file ? (
                <button
                  className="folder-toggle"
                  aria-expanded={!collapsed.has(path)}
                  onClick={() =>
                    setCollapsed((s) => {
                      const n = new Set(s);
                      if (n.has(path)) n.delete(path);
                      else n.add(path);
                      return n;
                    })
                  }
                >
                  {collapsed.has(path) ? (
                    <ChevronRight size={14} />
                  ) : (
                    <ChevronDown size={14} />
                  )}
                  <Folder size={15} />
                  <span>{name}</span>
                </button>
              ) : (
                <span className="file-name">
                  <File size={14} />
                  {name}
                </span>
              )}
              <span className="mono muted">
                {bytes(file?.size ?? nested.reduce((a, f) => a + f.size, 0))}
              </span>
            </div>
            {!file && !collapsed.has(path) && (
              <div className="nested">
                <FileTree
                  files={files}
                  selected={selected}
                  setSelected={setSelected}
                  prefix={path + "/"}
                  t={t}
                />
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
