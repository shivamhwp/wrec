// Entry point. wrec lives in the menu bar — a recorder belongs next to the
// clock, not in the dock. One `MenuBarExtra` (window style) is the entire
// control surface; Settings holds the rare knobs.
//
// Performance: the app renders nothing until the menu bar item is clicked.
// At idle there are zero timers, zero polls, zero animations — the process
// sits at 0.0% CPU with a handful of MB, which is the whole point of wrec.

import AppKit
import SwiftUI

@main
struct WrecApp: App {
    @State private var model: RecorderModel

    init() {
        if Smoke.requested {
            Task { await Smoke.run() }
        }
        Theme.registerFonts()
        let model = RecorderModel()
        _model = State(initialValue: model)
        #if DEBUG
        UIPreview.openIfRequested(model: model)
        #endif
    }

    var body: some Scene {
        MenuBarExtra {
            PopoverView(model: model)
        } label: {
            MenuBarLabel(model: model)
                #if DEBUG
                .background(StatusItemFrameReporter())
                #endif
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView(model: model)
        }
    }
}

#if DEBUG
/// `WREC_UI_PREVIEW=1`: host the popover content in a plain window at a
/// fixed position so automated tests can screenshot and click it without
/// touching the menu bar. Debug builds only.
@MainActor
enum UIPreview {
    static func openIfRequested(model: RecorderModel) {
        guard ProcessInfo.processInfo.environment["WREC_UI_PREVIEW"] == "1" else { return }
        DispatchQueue.main.async {
            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 320, height: 640),
                styleMask: [.titled],
                backing: .buffered,
                defer: false
            )
            window.title = "wrec preview"
            window.contentView = NSHostingView(rootView: PopoverView(model: model))
            window.level = .floating
            // Pin to a known spot: 40pt from the screen's top-left.
            if let screen = NSScreen.main {
                let top = screen.frame.maxY - 40
                window.setFrameTopLeftPoint(NSPoint(x: 40, y: top))
            }
            window.makeKeyAndOrderFront(nil)
            NSApp.activate(ignoringOtherApps: true)
            // The content sizes itself; print the final frame (both Cocoa
            // bottom-left and CG top-left coords) for the click driver.
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                let frame = window.frame
                let screenH = NSScreen.main?.frame.height ?? 0
                print(
                    "WREC_PREVIEW_FRAME cocoa=\(frame) cg_top_left=(\(frame.origin.x),\(screenH - frame.maxY)) size=(\(frame.width)x\(frame.height))"
                )
            }
        }
    }
}

/// Prints the menu bar status item's window frame so a click driver can hit
/// it precisely. Active only with `WREC_UI_DEBUG=1` in debug builds.
struct StatusItemFrameReporter: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        guard ProcessInfo.processInfo.environment["WREC_UI_DEBUG"] == "1" else { return view }
        DispatchQueue.main.async {
            if let window = view.window {
                let frame = window.frame
                let screenH = NSScreen.main?.frame.height ?? 0
                print(
                    "WREC_STATUSITEM_FRAME cocoa=\(frame) cg_center=(\(frame.midX),\(screenH - frame.midY))"
                )
            }
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {}
}
#endif

/// The menu bar item: a record glyph, plus the live timer while recording.
/// `menuBarText` is precomputed in the model and changes at most once a
/// second, so this view re-renders only on real state changes.
struct MenuBarLabel: View {
    let model: RecorderModel

    var body: some View {
        HStack(spacing: 3) {
            Image(systemName: symbol)
            if !model.menuBarText.isEmpty {
                Text(model.menuBarText)
                    .font(.system(size: 11, weight: .semibold).monospacedDigit())
            }
        }
    }

    private var symbol: String {
        switch model.phase {
        case .recording, .stopping: "record.circle.fill"
        case .paused, .pausing, .resuming: "pause.circle.fill"
        default: "record.circle"
        }
    }
}
