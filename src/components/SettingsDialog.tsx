import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";
import type { AccountInfo, AppSnapshot, Config } from "../lib/types";

const DEFAULT_MODELS = [
  { name: "unspecified", displayName: "自动选择", advancedOnly: false },
  { name: "gemini-3.8-flash", displayName: "Gemini 3.8 Flash", advancedOnly: false },
  { name: "gemini-3-flash", displayName: "Gemini 3 Flash", advancedOnly: false },
  { name: "gemini-3-pro", displayName: "Gemini 3 Pro", advancedOnly: false },
];

interface Props {
  snapshot: AppSnapshot;
  onClose: () => void;
  onSaved: (s: AppSnapshot) => void;
}

export function SettingsDialog({ snapshot, onClose, onSaved }: Props) {
  const [cfg, setCfg] = useState<Config>(snapshot.config);
  const [busy, setBusy] = useState<string | null>(null);
  const [msg, setMsg] = useState<{ kind: "ok" | "bad"; text: string } | null>(null);
  const [cookieText, setCookieText] = useState("");
  const [account, setAccount] = useState<AccountInfo | null>(null);

  const patch = (p: Partial<Config>) => setCfg({ ...cfg, ...p });

  async function save() {
    setBusy("save");
    try {
      await api.saveConfig(cfg);
      const next = await api.appSnapshot();
      onSaved(next);
      setMsg({ kind: "ok", text: "已保存" });
    } catch (e) {
      setMsg({ kind: "bad", text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  async function install() {
    setBusy("install");
    setMsg(null);
    try {
      const p = await api.installCli();
      const next = await api.appSnapshot();
      onSaved(next);
      setMsg({ kind: "ok", text: `已安装到 ${p}` });
    } catch (e) {
      setMsg({ kind: "bad", text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  async function doImportCookies() {
    setBusy("cookies");
    setMsg(null);
    try {
      await api.importCookies(cookieText);
      const next = await api.appSnapshot();
      onSaved(next);
      setCookieText("");
      setMsg({ kind: "ok", text: "Cookie 已导入" });
    } catch (e) {
      setMsg({ kind: "bad", text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  async function checkLogin() {
    setBusy("login");
    setMsg(null);
    try {
      const info = await api.checkAccount();
      setAccount(info);
      setMsg({
        kind: info.loggedIn ? "ok" : "bad",
        text: info.loggedIn ? `已登录${info.user ? `：${info.user}` : ""}` : info.message,
      });
    } catch (e) {
      setMsg({ kind: "bad", text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  const models = account?.models?.length ? account.models : DEFAULT_MODELS;

  return (
    <div className="overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>设置</h2>
        <div className="sub">Gemini Draw 通过 gemini-web-cli 调用 Gemini Web，无需 API Key</div>

        <div className="field">
          <label>引擎 gemini-web-cli</label>
          <div className="kv">
            <span className="k">状态</span>
            <span className="v">
              <span className={`badge ${snapshot.cli.found ? "ok" : "bad"}`}>
                {snapshot.cli.found ? "可用" : "缺失"}
              </span>
              {snapshot.cli.version ? ` v${snapshot.cli.version}` : ""} · {snapshot.cli.source}
            </span>
          </div>
          <div className="kv">
            <span className="k">路径</span>
            <span className="v">{snapshot.cli.path ?? snapshot.cli.error ?? "-"}</span>
          </div>
          <div className="row" style={{ marginTop: 10 }}>
            <button onClick={install} disabled={busy !== null}>
              {busy === "install" ? "安装中…" : "自动安装 / 更新"}
            </button>
            <input
              style={{ flex: 1, minWidth: 180 }}
              placeholder="或手动指定可执行文件路径"
              value={cfg.cliPath ?? ""}
              onChange={(e) => patch({ cliPath: e.target.value || null })}
            />
            <button
              onClick={async () => {
                const p = await open({ multiple: false, directory: false });
                if (typeof p === "string") patch({ cliPath: p });
              }}
            >
              浏览
            </button>
          </div>
        </div>

        <div className="field">
          <label>Cookie（浏览器登录态）</label>
          <div className="kv">
            <span className="k">状态</span>
            <span className="v">
              <span className={`badge ${snapshot.hasCookies ? "ok" : "bad"}`}>
                {snapshot.hasCookies ? "已导入" : "未导入"}
              </span>
            </span>
          </div>
          <textarea
            style={{ width: "100%", minHeight: 70, marginTop: 8 }}
            placeholder="粘贴整条 Cookie：__Secure-1PSID=...; __Secure-1PSIDTS=..."
            value={cookieText}
            onChange={(e) => setCookieText(e.target.value)}
          />
          <div className="row" style={{ marginTop: 8 }}>
            <button
              className="primary"
              onClick={doImportCookies}
              disabled={busy !== null || !cookieText.trim()}
            >
              {busy === "cookies" ? "导入中…" : "导入"}
            </button>
            <button onClick={checkLogin} disabled={busy !== null}>
              {busy === "login" ? "检查中…" : "检查登录状态"}
            </button>
            <span className="hint">
              在浏览器打开 gemini.google.com，从开发者工具 Network 请求头里复制 Cookie
            </span>
          </div>
          {account && (
            <div className="kv" style={{ marginTop: 8 }}>
              <span className="k">账号</span>
              <span className="v">
                {account.user ?? "-"}
                {account.tier ? ` · ${account.tier}` : ""}
              </span>
            </div>
          )}
        </div>

        <div className="field">
          <label>生成</label>
          <div className="row">
            <select
              value={cfg.model}
              onChange={(e) => patch({ model: e.target.value })}
              style={{ minWidth: 200 }}
            >
              {models.map((m) => (
                <option key={m.name} value={m.name}>
                  {m.displayName}（{m.name}）
                </option>
              ))}
              {!models.some((m) => m.name === cfg.model) && (
                <option value={cfg.model}>{cfg.model}</option>
              )}
            </select>
          </div>
          <div className="row" style={{ marginTop: 8 }}>
            <input
              style={{ flex: 1, minWidth: 160 }}
              placeholder="输出目录（留空用默认）"
              value={cfg.outputDir ?? ""}
              onChange={(e) => patch({ outputDir: e.target.value || null })}
            />
            <button
              onClick={async () => {
                const p = await api.pickDirectory();
                if (p) patch({ outputDir: p });
              }}
            >
              选择
            </button>
          </div>
        </div>

        <div className="field">
          <label>网络（可选）</label>
          <div className="row">
            <input
              style={{ flex: 1, minWidth: 160 }}
              placeholder="代理，例如 http://127.0.0.1:7890"
              value={cfg.proxy ?? ""}
              onChange={(e) => patch({ proxy: e.target.value || null })}
            />
            <input
              style={{ width: 120 }}
              placeholder="账号序号"
              value={cfg.accountIndex ?? ""}
              onChange={(e) => {
                const v = e.target.value.trim();
                patch({ accountIndex: v === "" ? null : Number(v) || null });
              }}
            />
          </div>
        </div>

        <div className="field">
          <label>数据</label>
          <div className="kv">
            <span className="k">应用数据目录</span>
            <span className="v">{snapshot.dataDir}</span>
          </div>
          <div className="row" style={{ marginTop: 8 }}>
            <button onClick={() => api.openPath(snapshot.dataDir)}>打开</button>
            <span className="hint">版本 v{snapshot.appVersion}</span>
          </div>
        </div>

        {msg && (
          <div className={`banner ${msg.kind === "ok" ? "warn" : "error"}`}>{msg.text}</div>
        )}

        <div className="actions">
          <button onClick={onClose}>关闭</button>
          <button className="primary" onClick={save} disabled={busy !== null}>
            {busy === "save" ? "保存中…" : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}
