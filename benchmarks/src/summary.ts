import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import type { RunResult } from "./gates";

// Full run documents carry raw per-frame decode arrays, process samples, and
// event logs — ~2 MB per release run. Useful when debugging a run locally,
// far too heavy to commit per release. The summary keeps every verdict,
// gate, aggregate, and derived number and drops the raw streams; summaries
// live in results/ (tracked by git) and are what a release publishes.

const missingIndexCap = 25;

type SlimDecode = {
  codec: string;
  dimensions: { width: number; height: number };
  durationMs: number;
  frameCount: number;
};

export type SlimRun = Omit<
  RunResult,
  | "recordingFilesDeleted"
  | "metrics"
  | "processSamples"
  | "jsonEvents"
  | "stdoutLines"
  | "stderrLines"
  | "decode"
> & {
  decode: SlimDecode | null;
};

const slimRun = (run: RunResult): SlimRun => {
  const {
    recordingFilesDeleted: _files,
    metrics: _metrics,
    processSamples: _samples,
    jsonEvents: _events,
    stdoutLines: _stdout,
    stderrLines: _stderr,
    decode,
    observed,
    ...rest
  } = run;

  return {
    ...rest,
    decode: decode
      ? {
          codec: decode.codec,
          dimensions: decode.dimensions,
          durationMs: decode.durationMs,
          frameCount: decode.frames.length,
        }
      : null,
    observed: observed
      ? {
          ...observed,
          missingStimulusIndices: observed.missingStimulusIndices.slice(0, missingIndexCap),
        }
      : null,
  };
};

export const slimResult = <T extends { profiles: any[] }>(result: T): T => ({
  ...result,
  profiles: result.profiles.map((profile) => ({
    ...profile,
    warmups: profile.warmups?.map(slimRun),
    measured: {
      candidate: profile.measured.candidate.map(slimRun),
      reference: profile.measured.reference?.map(slimRun),
    },
  })),
});

export const writeSummary = async (
  resultsDir: string,
  result: { id: string; profiles: any[] },
) => {
  await mkdir(resultsDir, { recursive: true });
  const summaryPath = path.join(resultsDir, `${result.id}.json`);
  await writeFile(summaryPath, `${JSON.stringify(slimResult(result), null, 2)}\n`);
  return summaryPath;
};

// One-off use: `bun src/summary.ts runs/<id>.json` re-derives the committed
// summary from a full run document.
if (import.meta.main) {
  const source = Bun.argv[2];
  if (!source) {
    console.error("usage: bun src/summary.ts <runs/full-run.json>");
    process.exit(1);
  }
  const full = JSON.parse(await Bun.file(source).text());
  const resultsDir = path.resolve(import.meta.dir, "..", "results");
  console.log(`summary: ${await writeSummary(resultsDir, full)}`);
}
