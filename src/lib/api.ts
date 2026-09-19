import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AccountInfo,
  AppSnapshot,
  Config,
  ProgressEvent,
  RefImage,
  ResultImage,
  Task,
} from "./types";

export const api = {
  appSnapshot: () => invoke<AppSnapshot>("app_snapshot"),

  saveConfig: (config: Config) => invoke<void>("save_config", { config }),

  installCli: () => invoke<string>("install_cli"),

  importCookies: (cookieText: string) =>
    invoke<void>("import_cookies", { cookieText }),

  checkAccount: () => invoke<AccountInfo>("check_account"),

  loadTasks: () => invoke<Task[]>("load_tasks"),

  saveTasks: (tasks: Task[]) => invoke<void>("save_tasks", { tasks }),

  importRefFile: (taskId: string, src: string) =>
    invoke<RefImage>("import_ref_file", { taskId, src }),

  importRefBase64: (taskId: string, name: string, data: string) =>
    invoke<RefImage>("import_ref_base64", { taskId, name, data }),

  removeRef: (taskId: string, refId: string) =>
    invoke<void>("remove_ref", { taskId, refId }),

  generateTask: (args: {
    taskId: string;
    title: string;
    prompt: string;
    model: string;
    refs: string[];
  }) => invoke<ResultImage[]>("generate_task", args),

  readImage: (path: string, maxSide?: number) =>
    invoke<string>("read_image", { path, maxSide: maxSide ?? null }),

  openPath: (path: string) => invoke<void>("open_path", { path }),

  pickDirectory: () => invoke<string | null>("pick_directory"),

  saveImageAs: (src: string, defaultName: string) =>
    invoke<string | null>("save_image_as", { src, defaultName }),
};

export function onProgress(cb: (e: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>("gemini-draw://progress", (e) => cb(e.payload));
}

export function onCliInstall(cb: (msg: string) => void): Promise<UnlistenFn> {
  return listen<string>("gemini-draw://cli-install", (e) => cb(e.payload));
}
