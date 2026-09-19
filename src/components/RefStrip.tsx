import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";
import type { RefImage } from "../lib/types";
import { Thumb } from "./Thumb";

interface Props {
  taskId: string;
  refs: RefImage[];
  onAdd: (images: RefImage[]) => void;
  onRemove: (refId: string) => void;
  onReorder: (from: number, to: number) => void;
}

export function RefStrip({ taskId, refs, onAdd, onRemove, onReorder }: Props) {
  const [hot, setHot] = useState(false);
  const [dragIdx, setDragIdx] = useState<number | null>(null);
  const [overIdx, setOverIdx] = useState<number | null>(null);

  async function pickFiles() {
    const picked = await open({
      multiple: true,
      directory: false,
      filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"] }],
    });
    const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
    if (paths.length === 0) return;
    const added: RefImage[] = [];
    for (const p of paths) {
      try {
        added.push(await api.importRefFile(taskId, p));
      } catch (e) {
        console.error(e);
      }
    }
    if (added.length) onAdd(added);
  }

  return (
    <div>
      <div
        className={`dropzone ${hot ? "hot" : ""}`}
        onClick={pickFiles}
        onDragOver={(e) => {
          e.preventDefault();
          setHot(true);
        }}
        onDragLeave={() => setHot(false)}
        onDrop={async (e) => {
          e.preventDefault();
          setHot(false);
          const files = Array.from(e.dataTransfer.files);
          const added: RefImage[] = [];
          for (const f of files) {
            if (!f.type.startsWith("image/") && !/\.(png|jpe?g|webp|gif|bmp)$/i.test(f.name)) {
              continue;
            }
            const b64 = await new Promise<string>((resolve, reject) => {
              const r = new FileReader();
              r.onload = () => resolve(String(r.result));
              r.onerror = () => reject(new Error("read failed"));
              r.readAsDataURL(f);
            });
            added.push(await api.importRefBase64(taskId, f.name, b64));
          }
          if (added.length) onAdd(added);
        }}
      >
        点击选择图片，或把图片拖到这里
        <div className="hint" style={{ marginTop: 4 }}>
          顺序会影响生成结果 —— 拖动画面可调整顺序
        </div>
      </div>

      {refs.length > 0 && (
        <div className="refs">
          {refs.map((r, i) => (
            <div
              key={r.id}
              className={`ref-card ${dragIdx === i ? "dragging" : ""} ${overIdx === i ? "over" : ""}`}
              draggable
              onDragStart={() => setDragIdx(i)}
              onDragEnd={() => {
                setDragIdx(null);
                setOverIdx(null);
              }}
              onDragOver={(e) => {
                e.preventDefault();
                setOverIdx(i);
              }}
              onDrop={(e) => {
                e.preventDefault();
                if (dragIdx !== null && dragIdx !== i) onReorder(dragIdx, i);
                setDragIdx(null);
                setOverIdx(null);
              }}
            >
              <Thumb path={r.path} maxSide={240} className="thumb" />
              <span className="idx">{i + 1}</span>
              <button
                className="del"
                title="移除"
                onClick={(e) => {
                  e.stopPropagation();
                  onRemove(r.id);
                }}
              >
                ✕
              </button>
              <div className="nm" title={r.name}>
                {r.name}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
