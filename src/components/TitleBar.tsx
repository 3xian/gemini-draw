import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();

interface Props {
  onOpenSettings: () => void;
}

export function TitleBar({ onOpenSettings }: Props) {
  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="brand" data-tauri-drag-region>
        <span className="dot" />
        Gemini Draw
      </div>
      <div className="spacer" data-tauri-drag-region />
      <button className="win-btn" title="设置" onClick={onOpenSettings}>
        ⚙
      </button>
      <button className="win-btn" title="最小化" onClick={() => win.minimize()}>
        ─
      </button>
      <button className="win-btn" title="最大化" onClick={() => win.toggleMaximize()}>
        ▢
      </button>
      <button className="win-btn close" title="关闭" onClick={() => win.close()}>
        ✕
      </button>
    </div>
  );
}
