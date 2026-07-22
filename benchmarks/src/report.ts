import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import type { Gate, OverallStatus, ProfileResult, SuiteName } from "./gates";

// Renders benchmarks/results/ (the committed run summaries) into a single
// dependency-free HTML page: newest run in full, older runs as a history
// table. Verdicts use the reserved status palette and always pair an icon
// with a label, so state never rides on color alone.

type Summary = {
  schema: string;
  id: string;
  generatedAt: string;
  suite: SuiteName;
  status: OverallStatus;
  duration: string;
  binaries: { candidate: string; reference?: string };
  git: { branch: string; commit: string; dirty: boolean };
  machine: { arch: string; cpu?: string; cpuCount?: number; totalMemoryBytes?: number };
  environment: {
    macos?: { productVersion: string };
    chip?: string;
    memoryBytes?: number;
    loadAverage?: number[];
    guards?: Array<{ name: string; status: string; measured: unknown }>;
  };
  profiles: ProfileResult[];
};

const root = path.resolve(import.meta.dir, "..");
const resultsDir = path.join(root, "results");

const statusMeta: Record<string, { icon: string; word: string; tone: string }> = {
  pass: { icon: "✓", word: "pass", tone: "good" },
  fail: { icon: "✕", word: "fail", tone: "critical" },
  inconclusive: { icon: "◐", word: "inconclusive", tone: "warning" },
  skipped: { icon: "–", word: "skipped", tone: "muted" },
};

export const writeReport = async () => {
  const summaries = await loadSummaries();
  const html = render(summaries);
  const reportPath = path.join(root, "index.html");
  await writeFile(reportPath, html);
  return reportPath;
};

const loadSummaries = async (): Promise<Summary[]> => {
  let names: string[] = [];
  try {
    names = (await readdir(resultsDir)).filter((name) => name.endsWith(".json"));
  } catch {
    return [];
  }
  const parsed = await Promise.all(
    names.map(async (name) => {
      try {
        return JSON.parse(await readFile(path.join(resultsDir, name), "utf8")) as Summary;
      } catch {
        return null;
      }
    }),
  );
  return parsed
    .filter((item): item is Summary => item !== null && typeof item?.id === "string")
    .sort((a, b) => b.id.localeCompare(a.id));
};

const chip = (status: string) => {
  const meta = statusMeta[status] ?? statusMeta.skipped;
  return `<span class="chip chip-${meta.tone}"><span aria-hidden="true">${meta.icon}</span>${meta.word}</span>`;
};

const fmt = (value: unknown, digits = 2): string => {
  if (value === null || value === undefined) return "–";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") {
    if (!Number.isFinite(value)) return "–";
    return Number.isInteger(value) ? String(value) : value.toFixed(digits);
  }
  return escapeHtml(String(value));
};

const fmtBytes = (bytes: number | null | undefined) => {
  if (!bytes && bytes !== 0) return "–";
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024).toFixed(0)} KB`;
};

const fmtPercent = (ratio: number | null | undefined) =>
  ratio === null || ratio === undefined ? "–" : `${(ratio * 100).toFixed(1)}%`;

const escapeHtml = (value: string) =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");

const gateRow = (gate: Gate) => {
  const bytes = gate.name.includes("rss");
  return `
  <tr>
    <td>${escapeHtml(gate.name)}</td>
    <td>${chip(gate.status)}</td>
    <td class="num">${bytes && typeof gate.measured === "number" ? fmtBytes(gate.measured) : fmt(gate.measured)}</td>
    <td class="muted">${escapeHtml(gate.threshold)}</td>
    <td class="num">${gate.delta === null || gate.delta === undefined ? "–" : bytes ? fmtBytes(gate.delta) : fmt(gate.delta)}${
      gate.deltaPercent === null || gate.deltaPercent === undefined
        ? ""
        : ` <span class="muted">(${gate.deltaPercent >= 0 ? "+" : ""}${gate.deltaPercent.toFixed(1)}%)</span>`
    }</td>
    <td class="muted">${escapeHtml(gate.details ?? "")}</td>
  </tr>`;
};

const tile = (label: string, value: string, sub?: string) => `
  <div class="tile">
    <div class="tile-label">${escapeHtml(label)}</div>
    <div class="tile-value">${value}</div>
    ${sub ? `<div class="tile-sub">${sub}</div>` : ""}
  </div>`;

const profileSection = (profile: ProfileResult) => {
  const o = profile.observed;
  const tiles = [
    tile(
      "Observed fps",
      fmt(o?.effectiveFps),
      o?.stimulusAchievedFps ? `stimulus ${fmt(o.stimulusAchievedFps)} fps` : undefined,
    ),
    tile("Capture completeness", fmtPercent(o?.captureCompleteness)),
    tile("Start latency", `${fmt(profile.latency.startMs, 0)} <span class="unit">ms</span>`),
    tile("Finalize", `${fmt(profile.latency.finalizeMs, 0)} <span class="unit">ms</span>`),
    tile(
      "CPU avg / p95",
      `${fmt(profile.process.avgTotalCpuPercent, 1)} / ${fmt(profile.process.p95TotalCpuPercent, 1)}<span class="unit">%</span>`,
    ),
    tile("Peak RSS", fmtBytes(profile.process.maxTotalRssBytes)),
  ].join("");

  return `
  <section class="card">
    <header class="card-head">
      <h3>${escapeHtml(profile.name)}</h3>
      ${chip(profile.status)}
    </header>
    <div class="tiles">${tiles}</div>
    ${
      profile.gates.length
        ? `<table>
        <thead><tr><th>Gate</th><th>Status</th><th class="num">Measured</th><th>Threshold</th><th class="num">Δ vs reference</th><th>Notes</th></tr></thead>
        <tbody>${profile.gates.map(gateRow).join("")}</tbody>
      </table>`
        : `<p class="muted">Smoke run — recorded ungated.</p>`
    }
  </section>`;
};

const latestSection = (latest: Summary) => {
  const guards = latest.environment.guards ?? [];
  const env = [
    latest.environment.chip,
    latest.environment.macos ? `macOS ${latest.environment.macos.productVersion}` : null,
    `${latest.git.commit.slice(0, 7)}${latest.git.dirty ? " (dirty)" : ""}`,
    latest.suite === "release" && latest.binaries.reference ? "A/B vs reference" : latest.suite,
    latest.duration,
  ]
    .filter(Boolean)
    .map((item) => `<span>${escapeHtml(String(item))}</span>`)
    .join("<span class='dot'>·</span>");

  return `
  <section class="verdict">
    <div class="verdict-chip">${chip(latest.status)}</div>
    <div>
      <h2>${escapeHtml(latest.id)}</h2>
      <p class="meta">${env}</p>
      <p class="meta">${guards
        .map((guard) => `${chip(guard.status)} <span class="muted">${escapeHtml(guard.name)}</span>`)
        .join(" &nbsp; ")}</p>
    </div>
  </section>
  ${latest.profiles.map(profileSection).join("")}`;
};

const historyRow = (run: Summary) => `
  <tr>
    <td>${escapeHtml(run.id)}</td>
    <td>${chip(run.status)}</td>
    <td>${escapeHtml(run.suite)}</td>
    <td class="mono">${escapeHtml(run.git.commit.slice(0, 7))}${run.git.dirty ? "*" : ""}</td>
    <td>${run.binaries.reference ? "A/B" : "budgets"}</td>
    <td class="muted">${escapeHtml(run.generatedAt)}</td>
  </tr>`;

const render = (summaries: Summary[]) => {
  const latest = summaries[0];
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>wrec bench</title>
<style>
  :root {
    color-scheme: light;
    --surface: #fcfcfb; --plane: #f9f9f7;
    --ink: #0b0b0b; --ink-2: #52514e; --muted: #898781;
    --grid: #e1e0d9; --ring: rgba(11,11,11,0.10);
    --good: #0ca30c; --warning: #b97e00; --critical: #d03b3b;
    --good-wash: rgba(12,163,12,0.10); --warning-wash: rgba(250,178,25,0.14);
    --critical-wash: rgba(208,59,59,0.10); --muted-wash: rgba(137,135,129,0.12);
  }
  @media (prefers-color-scheme: dark) {
    :root {
      color-scheme: dark;
      --surface: #1a1a19; --plane: #0d0d0d;
      --ink: #ffffff; --ink-2: #c3c2b7; --muted: #898781;
      --grid: #2c2c2a; --ring: rgba(255,255,255,0.10);
      --good: #0ca30c; --warning: #fab219; --critical: #d03b3b;
    }
  }
  * { box-sizing: border-box; }
  body {
    margin: 0; padding: 32px 20px 64px; background: var(--plane); color: var(--ink);
    font: 15px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif;
  }
  main { max-width: 980px; margin: 0 auto; display: grid; gap: 16px; }
  h1 { font-size: 20px; margin: 0; }
  h2 { font-size: 16px; margin: 0; }
  h3 { font-size: 15px; margin: 0; font-weight: 600; }
  .sub { color: var(--ink-2); margin: 2px 0 12px; }
  .verdict {
    display: flex; gap: 16px; align-items: center;
    background: var(--surface); border: 1px solid var(--ring); border-radius: 10px; padding: 18px 20px;
  }
  .verdict-chip .chip { font-size: 15px; padding: 6px 14px; }
  .meta { margin: 4px 0 0; color: var(--ink-2); font-size: 13px; }
  .meta .dot { margin: 0 6px; color: var(--muted); }
  .card { background: var(--surface); border: 1px solid var(--ring); border-radius: 10px; padding: 16px 20px 20px; }
  .card-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
  .tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 10px; margin-bottom: 14px; }
  .tile { border: 1px solid var(--grid); border-radius: 8px; padding: 10px 12px; }
  .tile-label { font-size: 12px; color: var(--ink-2); }
  .tile-value { font-size: 22px; font-weight: 600; margin-top: 2px; }
  .tile-value .unit { font-size: 13px; font-weight: 400; color: var(--ink-2); margin-left: 1px; }
  .tile-sub { font-size: 12px; color: var(--muted); margin-top: 2px; }
  table { width: 100%; border-collapse: collapse; font-size: 13px; }
  th { text-align: left; color: var(--muted); font-weight: 500; padding: 6px 10px; border-bottom: 1px solid var(--grid); }
  td { padding: 6px 10px; border-bottom: 1px solid var(--grid); vertical-align: top; }
  tr:last-child td { border-bottom: none; }
  .num { font-variant-numeric: tabular-nums; text-align: right; }
  th.num { text-align: right; }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; }
  .muted { color: var(--muted); }
  .chip {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: 12px; font-weight: 600; padding: 2px 10px; border-radius: 999px;
    color: var(--ink);
  }
  .chip-good { background: var(--good-wash); color: var(--good); }
  .chip-critical { background: var(--critical-wash); color: var(--critical); }
  .chip-warning { background: var(--warning-wash); color: var(--warning); }
  .chip-muted { background: var(--muted-wash); color: var(--muted); }
</style>
</head>
<body>
<main>
  <div>
    <h1>wrec bench</h1>
    <p class="sub">Ground-truth recording performance — decoded from the output video, not self-reported.</p>
  </div>
  ${latest ? latestSection(latest) : `<p class="muted">No results yet — run <code>bun run bench</code>.</p>`}
  ${
    summaries.length > 1
      ? `<section class="card">
      <header class="card-head"><h3>History</h3></header>
      <table>
        <thead><tr><th>Run</th><th>Status</th><th>Suite</th><th>Commit</th><th>Mode</th><th>Generated</th></tr></thead>
        <tbody>${summaries.slice(1).map(historyRow).join("")}</tbody>
      </table>
    </section>`
      : ""
  }
</main>
</body>
</html>
`;
};

if (import.meta.main) {
  console.log(`report: ${await writeReport()}`);
}
