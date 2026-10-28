// What a running job says about how far it is, read from its output line by
// line. The Updates page shows it on the button of each app, as a line along
// its lower edge.
//
// For apt it is exact for each package, from the status lines apt writes when
// it is started with APT::Status-Fd (they are the same in every language):
//   dlstatus:2:78.1:...   the file being downloaded now (2); the percentage is
//                         the one of the whole download, so it is not used
//   pmstatus:nano:22.2:.. a step dpkg took on a package; the percentage is the
//                         one of the whole run, so the steps are counted instead
// The other tools print no status lines: firmware prints a percentage for the
// phase it is in, Flatpak and Snap print none, and then the button only says
// the stage.

export type JobPhase = "waiting" | "downloading" | "installing";

export interface JobProgress {
  phase: JobPhase;
  /** apt: the file being downloaded now, 0 before the first. */
  file: number;
  /** apt: the file each package comes in, from the download lines. */
  fileOf: Map<string, number>;
  /** apt: whether dpkg has started, which is when every download is over. */
  installing: boolean;
  /** apt: steps dpkg has reported for each package. */
  steps: Map<string, number>;
  /** Last percentage printed by the tool, if any (firmware: within its current phase). */
  percent: number | null;
}

export const newProgress = (): JobProgress => ({
  phase: "waiting",
  file: 0,
  fileOf: new Map(),
  installing: false,
  steps: new Map(),
  percent: null,
});

/** Steps dpkg reports for one package: prepare, unpack, install, prepare to configure, configure, installed. */
const INSTALL_STEPS = 6;
/** The download of a package counts two steps: it starts and it ends. */
const DOWNLOAD_STEPS = 2;

const DOWNLOAD_STATUS = /^dlstatus:(\d+):/;
const INSTALL_STATUS = /^pmstatus:([^:]+)(?::[a-z0-9-]+)?:\d+(?:\.\d+)?:/;
// "Get:1 http://deb.debian.org/debian forky/main amd64 nano amd64 9.3-1 [707 kB]":
// only the first word is translated.
const FETCHED = /^\S+:(\d+) \S+ \S+ \S+ (\S+) \S+ \S+ \[/;
const PERCENT = /(\d{1,3})\s?%/g;
const DOWNLOADING = /download|fetch|pulling/i;
const INSTALLING = /install|updat|unpack|apply|applying|refresh|upgrad|deploy|commit|writing|verif|decompress/i;

/** The status lines apt writes for the progress are not part of its output. */
export const isStatusLine = (line: string): boolean => DOWNLOAD_STATUS.test(line) || INSTALL_STATUS.test(line);

/** Takes one more line of output into the progress of its job. */
export function track(progress: JobProgress, line: string): void {
  const download = DOWNLOAD_STATUS.exec(line);
  if (download) {
    progress.file = Number(download[1]);
    progress.phase = "downloading";
    return;
  }
  const install = INSTALL_STATUS.exec(line);
  if (install) {
    progress.installing = true;
    progress.phase = "installing";
    if (install[1] !== "dpkg-exec") progress.steps.set(install[1], (progress.steps.get(install[1]) ?? 0) + 1);
    return;
  }
  const fetched = FETCHED.exec(line);
  if (fetched) {
    progress.fileOf.set(fetched[2], Number(fetched[1]));
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
  /** 0 to 1, or null when the tool does not say how far it is: then there is no line, only the stage. */
  fraction: number | null;
}

/**
 * Where one app is, given the job that updates it. `group` is how many apps
 * of the same source the job updates (1 when it is the app alone).
 */
export function rowProgress(progress: JobProgress, source: string, id: string, group: number): RowProgress {
  if (source === "apt") {
    const steps = Math.min(progress.steps.get(id) ?? 0, INSTALL_STEPS);
    const file = progress.fileOf.get(id);
    // 0 not started, 1 downloading, 2 downloaded. Once dpkg starts every
    // download is over, and when nothing had to be downloaded it never began.
    const downloaded = progress.installing || (file !== undefined && file < progress.file);
    const download = downloaded ? 2 : file !== undefined && file === progress.file ? 1 : 0;
    const phase: JobPhase = steps > 0 ? "installing" : progress.installing ? "waiting" : download > 0 ? "downloading" : "waiting";
    return { phase, fraction: (download + steps) / (DOWNLOAD_STEPS + INSTALL_STEPS) };
  }
  // One percentage cannot stand for several apps, so with more than one the
  // button only says the stage.
  if (group === 1 && progress.percent !== null) return { phase: progress.phase, fraction: progress.percent / 100 };
  return { phase: progress.phase, fraction: null };
}
