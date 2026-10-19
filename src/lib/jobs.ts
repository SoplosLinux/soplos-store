import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { _ } from "svelte-i18n";
import { busy } from "./busy";
import { settings } from "./settings";
import type { SourceKind } from "./api";

export type JobAction =
  | "install"
  | "remove"
  | "update"
  | "install-file"
  | "refresh"
  | "check"
  | "autoremove"
  | "clean"
  | "unused"
  | "testing-on"
  | "testing-off";

export interface JobRequest {
  action: JobAction;
  /** "all" for a job over every installed source ("check"). */
  source: SourceKind | "all";
  ids?: string[];
  scope?: "user" | "system";
  file?: string;
}

export interface Job {
  id: number;
  title: string;
  /** App the job belongs to, so its buttons can show it is busy. */
  key?: string;
  lines: string[];
  status: "running" | "done" | "failed" | "cancelled";
}

interface JobDone {
  id: number;
  success: boolean;
  cancelled: boolean;
}

const MAX_LINES = 400;

export const jobs = writable<Job[]>([]);

/** Installed state changed by jobs in this session, by "<appId>:<source>". */
export const overrides = writable<Record<string, boolean>>({});

const waiting = new Map<number, (ok: boolean) => void>();
// Output and results that arrive before start_job has returned the ID.
const earlyLines = new Map<number, string[]>();
const earlyDone = new Map<number, JobDone>();

let listening = false;

export async function initJobs() {
  if (listening) return;
  listening = true;
  await listen<{ id: number; line: string }>("job-line", ({ payload }) => {
    if (!get(jobs).some((j) => j.id === payload.id)) {
      earlyLines.set(payload.id, [...(earlyLines.get(payload.id) ?? []), payload.line]);
      return;
    }
    jobs.update((list) =>
      list.map((j) => (j.id === payload.id ? { ...j, lines: [...j.lines, payload.line].slice(-MAX_LINES) } : j)),
    );
  });
  await listen<JobDone>("job-done", ({ payload }) => finish(payload));
}

function finish(done: JobDone) {
  if (!get(jobs).some((j) => j.id === done.id)) {
    earlyDone.set(done.id, done);
    return;
  }
  const status = done.success ? "done" : done.cancelled ? "cancelled" : "failed";
  jobs.update((list) => list.map((j) => (j.id === done.id ? { ...j, status } : j)));
  busy.update((n) => n - 1);
  waiting.get(done.id)?.(done.success);
  waiting.delete(done.id);
}

/** Starts a job and resolves with true when it finished without errors. */
export async function runJob(title: string, request: JobRequest, key?: string): Promise<boolean> {
  let id: number;
  try {
    id = await invoke<number>("start_job", { request: { ...request, ids: request.ids ?? [] } });
  } catch (error) {
    jobs.update((list) => [...list.slice(-20), { id: -Date.now(), title, key, lines: [String(error)], status: "failed" }]);
    return false;
  }
  busy.update((n) => n + 1);
  jobs.update((list) => [...list.slice(-20), { id, title, key, lines: earlyLines.get(id) ?? [], status: "running" }]);
  earlyLines.delete(id);
  return new Promise((resolve) => {
    waiting.set(id, resolve);
    const done = earlyDone.get(id);
    if (done) {
      earlyDone.delete(id);
      finish(done);
    }
  });
}

/**
 * Checks every installed source (Flatpak, Snap and APT) and refreshes apt's
 * package lists ("apt update", asks for the password), then reads Debian's
 * metadata again. Without the refresh a new kernel or update is not seen
 * until the user runs "sudo apt update" in a terminal.
 */
export async function refreshLists(): Promise<boolean> {
  const ok = await runJob(get(_)("updates.refreshing"), { action: "check", source: "all" }, "refresh-lists");
  if (ok) await invoke("reload_apt");
  return ok;
}

/** Pending question shown by ScopeDialog: the app name and the answer callback. */
export const scopeQuestion = writable<{ name: string; answer: (scope: "user" | "system" | null) => void } | null>(null);

/** Flatpak install scope from Settings, asking when it is set to "ask". */
export function chooseScope(name: string): Promise<"user" | "system" | null> {
  const preferred = get(settings).flatpakScope;
  if (preferred !== "ask") return Promise.resolve(preferred);
  return new Promise((resolve) => {
    scopeQuestion.set({
      name,
      answer: (scope) => {
        scopeQuestion.set(null);
        resolve(scope);
      },
    });
  });
}
