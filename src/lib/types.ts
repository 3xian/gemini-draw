export interface Config {
  cliPath: string | null;
  model: string;
  outputDir: string | null;
  proxy: string | null;
  accountIndex: number | null;
}

export interface CliStatus {
  found: boolean;
  path: string | null;
  version: string | null;
  source: string;
  error: string | null;
}

export interface AppSnapshot {
  appVersion: string;
  dataDir: string;
  config: Config;
  cli: CliStatus;
  hasCookies: boolean;
}

export interface RefImage {
  id: string;
  name: string;
  path: string;
  size: number;
}

export interface ResultImage {
  id: string;
  url: string;
  path: string | null;
  createdAt: number;
}

export interface Task {
  id: string;
  title: string;
  prompt: string;
  model: string | null;
  refs: RefImage[];
  results: ResultImage[];
  chatId: string | null;
  status: "idle" | "running" | "done" | "error";
  error: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface ModelInfo {
  name: string;
  displayName: string;
  advancedOnly: boolean;
}

export interface AccountInfo {
  loggedIn: boolean;
  message: string;
  user: string | null;
  tier: string | null;
  models: ModelInfo[];
}

export interface ProgressEvent {
  taskId: string;
  stage: string;
  message: string;
}

export const TASK_STATUS: Record<string, string> = {
  idle: "待生成",
  running: "生成中",
  done: "已完成",
  error: "失败",
};

export function newTask(): Task {
  const now = Date.now();
  return {
    id: `${now.toString(36)}${Math.random().toString(36).slice(2, 8)}`,
    title: "未命名任务",
    prompt: "",
    model: null,
    refs: [],
    results: [],
    chatId: null,
    status: "idle",
    error: null,
    createdAt: now,
    updatedAt: now,
  };
}
