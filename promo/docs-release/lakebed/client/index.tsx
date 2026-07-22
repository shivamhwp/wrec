import { VIDEO_DATA_URL } from "./video-data";

export function App() {
  return (
    <main className="min-h-screen bg-neutral-950 px-4 py-10 text-neutral-100 sm:px-8">
      <section className="mx-auto max-w-6xl">
        <header className="mb-6 flex items-end justify-between gap-6">
          <div>
            <p className="font-mono text-sm uppercase tracking-[0.28em] text-red-500">
              wrec / new docs
            </p>
            <h1 className="mt-2 text-3xl font-semibold tracking-tight sm:text-5xl">
              Record the screen. Not the machine.
            </h1>
          </div>
          <a
            className="hidden border border-neutral-700 px-4 py-2 font-mono text-sm text-neutral-300 hover:border-red-500 hover:text-white sm:block"
            download="wrec-docs-release.mp4"
            href={VIDEO_DATA_URL}
          >
            download preview
          </a>
        </header>
        <div className="overflow-hidden border border-neutral-800 bg-black shadow-2xl shadow-red-950/30">
          <video
            className="aspect-video w-full bg-black"
            controls
            playsInline
            preload="metadata"
            src={VIDEO_DATA_URL}
          />
        </div>
        <footer className="mt-5 flex items-center justify-between font-mono text-xs uppercase tracking-widest text-neutral-500">
          <span>30 seconds / sound on</span>
          <a
            className="text-red-500 hover:text-red-400"
            href="https://wrec.app/docs"
          >
            wrec.app/docs ↗
          </a>
        </footer>
      </section>
    </main>
  );
}
