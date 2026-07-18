// Entry point. wrec lives in the menu bar — a recorder belongs next to the
// clock, not in the dock. One `MenuBarExtra` (window style) is the entire
// control surface; Settings holds the rare knobs.
//
// Performance: the app renders nothing until the menu bar item is clicked.
// At idle there are zero timers, zero polls, zero animations — the process
// sits at 0.0% CPU with a handful of MB, which is the whole point of wrec.

import SwiftUI

@main
struct WrecApp: App {
    @State private var model: RecorderModel

    init() {
        if Smoke.requested {
            Task { await Smoke.run() }
        }
        Theme.registerFonts()
        _model = State(initialValue: RecorderModel())
    }

    var body: some Scene {
        MenuBarExtra {
            PopoverView(model: model)
        } label: {
            MenuBarLabel(model: model)
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView(model: model)
        }
    }
}

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
