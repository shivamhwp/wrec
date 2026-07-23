import Foundation

/// Exercises the complete updater against a local release archive: discovery,
/// digest verification, extraction, signature/identity validation, daemon
/// stop, and the atomic bundle swap. The preview script launches a disposable
/// packaged app with this hook; production behavior is unchanged.
enum UpdateSmoke {
    static var requested: Bool {
        ProcessInfo.processInfo.environment["WREC_UPDATE_SMOKE"] == "1"
    }

    static func run() async -> Never {
        setbuf(stdout, nil)
        do {
            guard let release = try await Updater.check() else {
                throw UpdaterError.message("local update was not newer than the installed app")
            }
            let ready = try await Updater.downloadAndApply(release, daemon: DaemonClient())
            let info = ready.bundle.appending(path: "Contents/Info.plist")
            guard
                let dictionary = NSDictionary(contentsOf: info),
                dictionary["CFBundleShortVersionString"] as? String == release.version
            else {
                throw UpdaterError.message("swapped bundle has the wrong version")
            }
            try? FileManager.default.removeItem(at: ready.oldBundle)
            print("update-smoke: PASS \(Updater.currentVersion) -> \(release.version)")
            exit(0)
        } catch {
            print("update-smoke: FAIL \(error)")
            exit(1)
        }
    }
}
