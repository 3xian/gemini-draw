import { api } from "../lib/api";
import type { ResultImage } from "../lib/types";
import { Thumb } from "./Thumb";

interface Props {
  results: ResultImage[];
  onZoom: (r: ResultImage) => void;
}

function dirOf(path: string) {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : path;
}

export function ResultGrid({ results, onZoom }: Props) {
  if (results.length === 0) {
    return <div className="hint">还没有生成结果。</div>;
  }
  return (
    <div className="results">
      {results.map((r) => (
        <div className="result-card" key={r.id}>
          {r.path ? (
            <Thumb
              path={r.path}
              maxSide={640}
              className="thumb"
              onClick={() => onZoom(r)}
            />
          ) : (
            <div
              className="thumb"
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--muted-2)",
                fontSize: 12,
                padding: 10,
                textAlign: "center",
              }}
              title={r.url}
            >
              未下载到本地
              <br />
              <span style={{ fontSize: 10 }}>{r.url.slice(0, 40)}…</span>
            </div>
          )}
          <div className="bar">
            <button disabled={!r.path} onClick={() => r.path && onZoom(r)}>
              查看
            </button>
            <button
              disabled={!r.path}
              onClick={async () => {
                if (!r.path) return;
                const name = r.path.split(/[\\/]/).pop() ?? "gemini.png";
                await api.saveImageAs(r.path, name);
              }}
            >
              另存为
            </button>
            <button
              disabled={!r.path}
              onClick={() => r.path && api.openPath(dirOf(r.path))}
            >
              文件夹
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
