// What a running job says about how far it is, read from its output line by
// line: whether it is still waiting for the password, downloading or
// installing, which packages apt has already set up and, when the tool
// prints one, a percentage. The Updates page shows it on the button of each
// app and in a thin bar under its row.

export type JobPhase = "waiting" | "downloading" | "installing";

export interface JobProgress {
  phase: JobPhase;
  /** apt: "Get:" lines seen, one per file downloaded. */
  downloads: number;
  /** apt: packages already set up. */
  setups: Set<string>;
  /** Last percentage printed by the tool, if any. */
  percent: number | null;
}

export const newProgress = (): JobProgress => ({ phase: "waiting", downloads: 0, setups: new Set(), percent: null });

const APT_GET = /^Get:\d+/;
const APT_INSTALLING = /^(Preparing to unpack|Unpacking|Selecting previously|Setting up|Processing triggers)/;
const APT_SETUP = /^Setting up ([^\s:]+)/;
const PERCENT = /(\d{1,3})\s?%/g;
const DOWNLOADING = /download|fetch|pulling/i;
const INSTALLING = /install|updat|unpack|apply|applying|refresh|upgrad|deploy|commit|writing|verif|decompress/i;

/** Takes one more line of output into the progress of its job. */
export function track(progress: JobProgress, line: string): void {
  if (APT_GET.test(line)) {
    progress.downloads += 1;
    if (progress.phase === "waiting") progress.phase = "downloading";
    return;
  }
  if (APT_INSTALLING.test(line)) {
    progress.phase = "installing";
    const setup = APT_SETUP.exec(line);
    if (setup) progress.setups.add(setup[1]);
    return;
  }
  const percents = [...line.matchAll(PERCENT)].map((m) => Number(m[1])).filter((n) => n <= 100);
  if (percents.length) progress.percent = percents[percents.length - 1];
  if (DOWNLOADING.test(line)) progress.phase = "downloading";
  else if (INSTALLING.test(line)) progress.phase = "installing";
  else if (progress.phase === "waiting") progress.phase = "downloading";
}

export interface RowProgress {
  /** "queued" waits for the job before it; "done" has finished and is waiting for the list to be read again. */
  phase: JobPhase | "queued" | "done";
  /** 0 to 1. */
  fraction: number;
}

/**
 * Where one app is, given the job that updates it. `group` is how many apps
 * of the same source the job updates (1 when it is the app alone).
 */
export function rowProgress(progress: JobProgress, source: string, id: string, group: number): RowProgress {
  const count = Math.max(group, 1);
  if (source === "apt") {
    if (progress.setups.has(id)) return { phase: "installing", fraction: 1 };
    if (progress.phase === "installing") return { phase: "installing", fraction: 0.5 + 0.5 * Math.min(progress.setups.size / count, 1) };
    if (progress.phase === "downloading") return { phase: "downloading", fraction: 0.5 * Math.min(progress.downloads / count, 1) };
    return { phase: "waiting", fraction: 0 };
  }
  // Other tools: the percentage they print when it is one app, otherwise the
  // stage they are in, since one percentage cannot stand for several apps.
  if (count === 1 && progress.percent !== null) return { phase: progress.phase, fraction: progress.percent / 100 };
  const stage = { waiting: 0, downloading: 0.25, installing: 0.65 }[progress.phase];
  return { phase: progress.phase, fraction: stage };
}
