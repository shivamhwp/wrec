# wrec docs site

Astro site for the public wrec landing page and docs.

## Commands

Run commands from this directory.

```sh
bun install
bun run dev
bun run format
bun run check
```

Do not use npm, pnpm, yarn, or npx in this project.

## Deployment

The Vercel project root is this `docs` directory. Vercel detects Astro and
owns the install, build, and output settings; do not configure commands or a
`dist` directory manually. Keep source files outside the root directory
available during builds so the benchmark pages can read
`../benchmarks/results`.

## Pages

- `src/pages/index.astro` is the minimal landing page.
- `src/pages/docs/` documents the agent CLI contract and runtime architecture.

The docs should stay aligned with the native shell in `apps/mac`, the shared
protocol in `crates/control`, and the agent-facing CLI in `crates/cli`.
