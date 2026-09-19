import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, onProgress } from "./lib/api";
import {
  newTask,
  TASK_STATUS,
  type AppSnapshot,
  type ProgressEvent,
  type RefImage,
  type ResultImage,
  type Task,
} from "./lib/types";
import { Lightbox } from "./components/Lightbox";
import { RefStrip } from "./components/RefStrip";
import { ResultGrid } from "./components/ResultGrid";
import { SettingsDialog } from "./components/SettingsDialog";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";

const IMAGE_RE = /\.(png|jpe?g|webp|gif|bmp|avif)$/i;

export default function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [progress, setProgress] = useState<Record<string, ProgressEvent>>({});
  const [error, setError] = useState<string | null>(null);
  const [zoom, setZoom] = useState<ResultImage | null>(null);
  const [loaded, setLoaded] = useState(false);

  const activeIdRef = useRef<string | null>(null);
  activeIdRef.current = activeId;

  const patchTask = useCallback((id: string, patch: Partial<Task>) => {
    setTasks((prev) =>
      prev.map((t) => (t.id === id ? { ...t, ...patch, updatedAt: Date.now() } : t)),
    );
  }, []);

  const addRefs = useCallback(
    (id: string, images: RefImage[]) => {
      setTasks((prev) =>
        prev.map((t) =>
          t.id === id ? { ...t, refs: [...t.refs, ...images], updatedAt: Date.now() } : t,
        ),
      );
    },
    [],
  );

  useEffect(() => {
    (async () => {
      try {
        const s = await api.appSnapshot();
        setSnapshot(s);
        const t = await api.loadTasks();
        setTasks(t);
        setActiveId(t[0]?.id ?? null);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoaded(true);
      }
    })();
  }, []);

  useEffect(() => {
    if (!loaded) return;
    const id = window.setTimeout(() => {
      api.saveTasks(tasks).catch((e) => console.error(e));
    }, 400);
    return () => window.clearTimeout(id);
  }, [tasks, loaded]);

  useEffect(() => {
    let stop: (() => void) | undefined;
    onProgress((e) => setProgress((p) => ({ ...p, [e.taskId]: e }))).then((f) => {
      stop = f;
    });
    return () => stop?.();
  }, []);

  // 从系统拖拽图片文件进来 → 加到当前任务
  useEffect(() => {
    let stop: (() => void) | undefined;
    getCurrentWindow()
      .onDragDropEvent((event) => {
        const p = event.payload as unknown as {
          type?: string;
          event?: string;
          paths?: string[];
        };
        const type = p?.type ?? p?.event ?? "";
        const paths = p?.paths ?? [];
        if (type === "leave") return;
        if (type !== "drop") return;
        const taskId = activeIdRef.current;
        if (!taskId) return;
        const files = paths.filter((x) => IMAGE_RE.test(x));
        (async () => {
          const added: RefImage[] = [];
          for (const f of files) {
            try {
              added.push(await api.importRefFile(taskId, f));
            } catch (e) {
              console.error(e);
            }
          }
          if (added.length) addRefs(taskId, added);
        })();
      })
      .then((f) => {
        stop = f;
      });
    return () => stop?.();
  }, [addRefs]);

  const active = useMemo(() => tasks.find((t) => t.id === activeId) ?? null, [tasks, activeId]);

  function createTask() {
    const t = newTask();
    setTasks((prev) => [t, ...prev]);
    setActiveId(t.id);
  }

  function deleteTask(id: string) {
    setTasks((prev) => prev.filter((t) => t.id !== id));
    if (activeId === id) setActiveId(null);
  }

  async function generate(task: Task) {
    if (!snapshot) return;
    if (!snapshot.cli.found) {
      setError("还没有可用的 gemini-web-cli，请在设置里安装或指定路径");
      setSettingsOpen(true);
      return;
    }
    if (!snapshot.hasCookies) {
      setError("还没有导入 Cookie，请先在设置里导入");
      setSettingsOpen(true);
      return;
    }
    if (!task.prompt.trim()) {
      setError("请先填写 Prompt");
      return;
    }
    setError(null);
    patchTask(task.id, { status: "running", error: null });
    try {
      const results = await api.generateTask({
        taskId: task.id,
        title: task.title,
        prompt: task.prompt,
        model: task.model ?? snapshot.config.model,
        refs: task.refs.map((r) => r.path),
      });
      setTasks((prev) =>
        prev.map((t) =>
          t.id === task.id
            ? { ...t, status: "done", results: [...results, ...t.results], updatedAt: Date.now() }
            : t,
        ),
      );
    } catch (e) {
      const msg = String(e);
      patchTask(task.id, { status: "error", error: msg });
      setError(msg);
    } finally {
      setProgress((p) => {
        const n = { ...p };
        delete n[task.id];
        return n;
      });
    }
  }

  return (
    <div className="app">
      <TitleBar onOpenSettings={() => setSettingsOpen(true)} />
      <div className="body">
        <Sidebar
          tasks={tasks}
          activeId={activeId}
          onSelect={setActiveId}
          onCreate={createTask}
          onDelete={deleteTask}
        />
        <div className="main">
          {!active ? (
            <div className="empty">
              {tasks.length === 0 ? "点击「新建任务」开始" : "选择左侧的任务"}
            </div>
          ) : (
            <div className="main-scroll">
              <div className="field">
                <input
                  className="title-input"
                  value={active.title}
                  placeholder="任务标题"
                  onChange={(e) => patchTask(active.id, { title: e.target.value })}
                />
              </div>

              <div className="field">
                <label>Prompt</label>
                <textarea
                  className="prompt-area"
                  value={active.prompt}
                  placeholder="描述你想生成的画面，例如：把这几张参考图融合成一张赛博朋克风格的城市夜景海报"
                  onChange={(e) => patchTask(active.id, { prompt: e.target.value })}
                />
              </div>

              <div className="field">
                <label>参考图（{active.refs.length}）· 可拖拽排序</label>
                <RefStrip
                  taskId={active.id}
                  refs={active.refs}
                  onAdd={(imgs) => addRefs(active.id, imgs)}
                  onRemove={async (refId) => {
                    await api.removeRef(active.id, refId).catch(() => {});
                    setTasks((prev) =>
                      prev.map((t) =>
                        t.id === active.id
                          ? {
                              ...t,
                              refs: t.refs.filter((r) => r.id !== refId),
                              updatedAt: Date.now(),
                            }
                          : t,
                      ),
                    );
                  }}
                  onReorder={(from, to) =>
                    setTasks((prev) =>
                      prev.map((t) => {
                        if (t.id !== active.id) return t;
                        const refs = [...t.refs];
                        const [moved] = refs.splice(from, 1);
                        refs.splice(to, 0, moved);
                        return { ...t, refs, updatedAt: Date.now() };
                      }),
                    )
                  }
                />
              </div>

              <div className="field row">
                <span className="hint">模型</span>
                <select
                  value={active.model ?? snapshot?.config.model ?? "unspecified"}
                  onChange={(e) => patchTask(active.id, { model: e.target.value })}
                  style={{ minWidth: 180 }}
                >
                  <option value="unspecified">自动选择（跟随设置）</option>
                  <option value="gemini-3.8-flash">Gemini 3.8 Flash</option>
                  <option value="gemini-3-flash">Gemini 3 Flash</option>
                  <option value="gemini-3-pro">Gemini 3 Pro</option>
                  {active.model &&
                    !["unspecified", "gemini-3.8-flash", "gemini-3-flash", "gemini-3-pro"].includes(
                      active.model,
                    ) && <option value={active.model}>{active.model}</option>}
                </select>
                <span style={{ flex: 1 }} />
                <button
                  className="primary"
                  disabled={active.status === "running"}
                  onClick={() => generate(active)}
                >
                  {active.status === "running" ? "生成中…" : "生成图片"}
                </button>
              </div>

              {progress[active.id] && active.status === "running" && (
                <div className="progress">
                  <span className="spinner" />
                  <span>
                    {progress[active.id].message}
                    <span className="hint"> · {TASK_STATUS[active.status]}</span>
                  </span>
                </div>
              )}

              {error && <div className="banner error">{error}</div>}
              {active.error && !error && <div className="banner error">{active.error}</div>}

              <div className="field">
                <label>生成结果（{active.results.length}）</label>
                <ResultGrid
                  results={active.results}
                  onZoom={(r) => r.path && setZoom(r)}
                />
              </div>
            </div>
          )}
        </div>
      </div>

      {settingsOpen && snapshot && (
        <SettingsDialog
          snapshot={snapshot}
          onClose={() => setSettingsOpen(false)}
          onSaved={(s) => {
            setSnapshot(s);
            setSettingsOpen(false);
          }}
        />
      )}

      {zoom?.path && <Lightbox path={zoom.path} onClose={() => setZoom(null)} />}
    </div>
  );
}
