import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { _ } from "svelte-i18n";
import { busy } from "./busy";
import { settings } from "./settings";
import { api, type SourceKind } from "./api";
import { newProgress, track, isStatusLine, type JobProgress } from "./updateProgress";

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
  /** "all" for a job over every installed source ("check"), "driver" for the Drivers page. */
  source: SourceKind | "all" | "driver";
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
  /** How far the job is, read from its output (see updateProgress.ts). */
  progress: JobProgress;
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
      list.map((j) => {
        if (j.id !== payload.id) return j;
        track(j.progress, payload.line);
        // The status lines apt writes for the progress are not part of its output.
        const lines = isStatusLine(payload.line) ? j.lines : [...j.lines, payload.line].slice(-MAX_LINES);
        return { ...j, lines, progress: j.progress };
      }),
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
    jobs.update((list) => [...list.slice(-20), { id: -Date.now(), title, key, lines: [String(error)], progress: newProgress(), status: "failed" }]);
    return false;
  }
  busy.update((n) => n + 1);
  const early = earlyLines.get(id) ?? [];
  const progress = newProgress();
  early.forEach((line) => track(progress, line));
  jobs.update((list) => [...list.slice(-20), { id, title, key, lines: early.filter((line) => !isStatusLine(line)), progress, status: "running" }]);
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
  // Debian's metadata is read again in the background: the page does not
  // wait for it, it only needs it for the names of apps it has not seen.
  if (ok) void invoke("reload_apt");
  return ok;
}

/** Pending question shown by ScopeDialog: the app name and the answer callback. */
export const scopeQuestion = writable<{ name: string; answer: (scope: "user" | "system" | null) => void } | null>(null);

/**
 * Flatpak install scope. When Flathub is set up in one installation only there
 * is nothing to choose; with both (or none) it is the one chosen in Settings,
 * asking when it is set to "ask".
 */
export async function chooseScope(name: string): Promise<"user" | "system" | null> {
  const status = await api.sources().catch(() => null);
  const flathub = status?.flathub;
  if (flathub === "user" || flathub === "system") return flathub;
  const preferred = get(settings).flatpakScope;
  if (preferred !== "ask") return preferred;
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
