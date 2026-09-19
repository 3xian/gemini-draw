import { TASK_STATUS, type Task } from "../lib/types";

interface Props {
  tasks: Task[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onCreate: () => void;
  onDelete: (id: string) => void;
}

export function Sidebar({ tasks, activeId, onSelect, onCreate, onDelete }: Props) {
  return (
    <div className="sidebar">
      <div className="sidebar-head">
        <button className="primary" onClick={onCreate}>
          + 新建任务
        </button>
      </div>
      <div className="task-list">
        {tasks.length === 0 && (
          <div style={{ padding: "8px 10px", color: "var(--muted-2)", fontSize: 12 }}>
            还没有任务
          </div>
        )}
        {tasks.map((t) => (
          <div
            key={t.id}
            className={`task-item ${t.id === activeId ? "active" : ""}`}
            onClick={() => onSelect(t.id)}
          >
            <div className="t">{t.title || "未命名任务"}</div>
            <div className="m">
              <span
                className={`badge ${t.status === "error" ? "bad" : t.status === "done" ? "ok" : ""}`}
              >
                {TASK_STATUS[t.status] ?? t.status}
              </span>
              <span>{t.refs.length} 参考图</span>
              <span>{t.results.length} 结果</span>
              <span style={{ flex: 1 }} />
              <button
                className="ghost"
                style={{ padding: "0 4px", fontSize: 11 }}
                title="删除任务"
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete(t.id);
                }}
              >
                ✕
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
