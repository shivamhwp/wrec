// Headless smoke test: `WREC_SMOKE=1 wrec-app` exercises the entire daemon
// wire — spawn, status, permissions, targets, start → poll → stop — and
// exits nonzero on any failure. No UI, no TCC prompts beyond what the daemon
// itself triggers. Used by scripts and CI to prove the shell ⇄ engine
// contract without a display.

import Foundation

enum Smoke {
    static var requested: Bool {
        ProcessInfo.processInfo.environment["WREC_SMOKE"] == "1"
    }

    static func run() async -> Never {
        setbuf(stdout, nil)
        let daemon = DaemonClient()
        do {
            try await daemon.ensure()
            let status = try await daemon.status()
            print("smoke: daemon \(status.daemonVersion) protocol \(status.protocolVersion)")

            let permission = try await daemon.screenPermissionStatus()
            print("smoke: screen permission \(permission.rawValue)")

            guard permission.isGranted else {
                print("smoke: PASS (stopping before record: no screen permission)")
                exit(0)
            }

            let targets = try await daemon.listTargets()
            print("smoke: \(targets.count) targets")
            guard let display = targets.first(where: { $0.kind == .display }) else {
                print("smoke: PASS (no display target)")
                exit(0)
            }

            var settings = RecorderSettings.defaults()
            settings.outputDir = FileManager.default.temporaryDirectory
                .appending(path: "wrec-smoke").path
            let job = try await daemon.startRecording(
                StartRecordingParams(target: display, settings: settings))
            print("smoke: job \(job.id) \(job.status.rawValue)")

            try await Task.sleep(for: .seconds(2))
            let live = try await daemon.showJob(job.id)
            print("smoke: polled \(live.status.rawValue) events=\(live.events.count)")

            let stopped = try await daemon.stopJob(job.id)
            print("smoke: stop → \(stopped.status.rawValue)")

            for _ in 0..<20 {
                let snapshot = try await daemon.showJob(job.id)
                if snapshot.status.isTerminal {
                    print(
                        "smoke: terminal \(snapshot.status.rawValue) output=\(snapshot.outputPath ?? "-")"
                    )
                    guard snapshot.status == .completed else {
                        print("smoke: FAIL (job did not complete)")
                        exit(1)
                    }
                    print("smoke: PASS")
                    exit(0)
                }
                try await Task.sleep(for: .milliseconds(500))
            }
            print("smoke: FAIL (job never reached terminal state)")
            exit(1)
        } catch {
            print("smoke: FAIL \(error)")
            exit(1)
        }
    }
}
