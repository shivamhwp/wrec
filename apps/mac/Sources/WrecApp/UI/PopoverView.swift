// The whole recorder in one 320pt panel. Structured so observation stays
// narrow: transport, config, and status are separate child views, each
// reading only the model properties it renders.

import SwiftUI

struct PopoverView: View {
    @Bindable var model: RecorderModel

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HeaderRow(model: model)
            Divider()

            if !model.screenPermission.isGranted {
                PermissionBanner(model: model)
                Divider()
            }

            TransportSection(model: model)
            Divider()
            ConfigSection(model: model)

            if let toast = model.toast {
                Divider()
                Text(toast)
                    .font(.pixel(10))
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 8)
            }
        }
        .frame(width: 320)
        .task { await model.refreshTargets() }
    }
}

// MARK: - Header

private struct HeaderRow: View {
    let model: RecorderModel

    var body: some View {
        HStack(spacing: 8) {
            Text("WREC")
                .font(.pixel(13))
            Circle()
                .fill(model.phase.isActiveSession ? Theme.red : Color.secondary.opacity(0.4))
                .frame(width: 6, height: 6)
            Spacer()
            SettingsLink {
                Image(systemName: "gearshape")
                    .imageScale(.medium)
            }
            .buttonStyle(.borderless)
            Button {
                model.quit()
            } label: {
                Image(systemName: "power")
                    .imageScale(.medium)
            }
            .buttonStyle(.borderless)
            .help("Quit wrec")
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
    }
}

// MARK: - Permission banner

private struct PermissionBanner: View {
    let model: RecorderModel

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.orange)
            Text("Screen Recording access needed")
                .font(.system(size: 12))
            Spacer()
            Button("Grant") {
                Task { await model.refreshScreenPermission(requestIfNeeded: true) }
            }
            .controlSize(.small)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }
}

// MARK: - Transport

private struct TransportSection: View {
    let model: RecorderModel

    var body: some View {
        VStack(spacing: 8) {
            HStack(spacing: 8) {
                if model.phase.isActiveSession {
                    PauseButton(model: model)
                }
                RecordButton(model: model)
            }
            if model.phase.isActiveSession, !model.metricsText.isEmpty {
                Text(model.metricsText)
                    .font(.pixel(11))
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
    }
}

private struct RecordButton: View {
    let model: RecorderModel

    var body: some View {
        Button(action: model.toggleRecording) {
            HStack(spacing: 8) {
                Image(systemName: model.phase.isActiveSession ? "stop.fill" : "record.circle.fill")
                    .font(.system(size: 13, weight: .bold))
                Text(label)
                    .font(.pixel(13))
            }
            .frame(maxWidth: .infinity)
            .frame(height: 40)
            .foregroundStyle(.white)
            .background(Theme.red, in: RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(.plain)
        .disabled(disabled)
        .opacity(disabled ? 0.45 : 1)
    }

    private var label: String {
        switch model.phase {
        case .starting: "STARTING"
        case .stopping: "STOPPING"
        default: model.phase.isActiveSession ? "STOP" : "REC"
        }
    }

    private var disabled: Bool {
        if model.phase.isActiveSession {
            return model.phase == .starting || model.phase == .stopping
        }
        return !model.canRecord
    }
}

private struct PauseButton: View {
    let model: RecorderModel

    var body: some View {
        Button(action: model.togglePause) {
            Image(systemName: model.phase == .paused ? "play.fill" : "pause.fill")
                .font(.system(size: 13, weight: .bold))
                .frame(width: 48, height: 40)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(.plain)
        .disabled(model.phase == .pausing || model.phase == .resuming || model.phase == .stopping)
    }
}

// MARK: - Config

private struct ConfigSection: View {
    @Bindable var model: RecorderModel

    private var locked: Bool {
        model.phase.isActiveSession || model.phase.isBusy
    }

    var body: some View {
        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 8) {
            GridRow {
                FieldLabel("SOURCE")
                Picker("", selection: sourceBinding) {
                    ForEach(CaptureSourceKind.allCases, id: \.self) { kind in
                        Text(kind.label).tag(kind)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
            }
            GridRow {
                FieldLabel("TARGET")
                HStack(spacing: 4) {
                    Picker("", selection: targetBinding) {
                        ForEach(model.visibleTargets) { target in
                            Text(target.name).tag(target.key)
                        }
                    }
                    .labelsHidden()
                    Button {
                        Task { await model.refreshTargets() }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                            .imageScale(.small)
                    }
                    .buttonStyle(.borderless)
                    .help("Refresh targets")
                }
            }
            GridRow {
                FieldLabel("FORMAT")
                enumPicker(\.codec, Codec.allCases) { $0.label }
            }
            GridRow {
                FieldLabel("PRESET")
                enumPicker(\.quality, Quality.allCases) { $0.label }
            }
            GridRow {
                FieldLabel("SIZE")
                enumPicker(
                    \.resolution, Resolution.allCases.filter(model.settings.resolutionAllowed)
                ) { $0.label }
            }
            GridRow {
                FieldLabel("FPS")
                enumPicker(\.fps, FrameRate.allCases.filter(model.settings.fpsAllowed)) {
                    $0.label
                }
            }
            GridRow {
                FieldLabel("CURSOR")
                toggle(\.includeCursor)
            }
            GridRow {
                FieldLabel("AUDIO")
                toggle(\.includeSystemAudio)
            }
            GridRow {
                FieldLabel("MIC")
                HStack(spacing: 6) {
                    toggle(\.includeMicrophone)
                    if model.settings.includeMicrophone, !model.micPermission.isGranted {
                        Button("Grant") {
                            Task { await model.requestMicPermission() }
                        }
                        .controlSize(.mini)
                    }
                }
            }
        }
        .disabled(locked)
        .controlSize(.small)
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
    }

    private var sourceBinding: Binding<CaptureSourceKind> {
        Binding(
            get: { model.settings.source },
            set: { value in model.update { $0.source = value } }
        )
    }

    private var targetBinding: Binding<String> {
        Binding(
            get: { model.selectedTarget?.key ?? "" },
            set: { model.select(targetKey: $0) }
        )
    }

    private func enumPicker<Value: Hashable>(
        _ keyPath: WritableKeyPath<RecorderSettings, Value>,
        _ options: [Value],
        label: @escaping (Value) -> String
    ) -> some View {
        Picker(
            "",
            selection: Binding(
                get: { model.settings[keyPath: keyPath] },
                set: { value in model.update { $0[keyPath: keyPath] = value } }
            )
        ) {
            ForEach(options, id: \.self) { option in
                Text(label(option)).tag(option)
            }
        }
        .labelsHidden()
        .frame(maxWidth: 140, alignment: .leading)
    }

    private func toggle(_ keyPath: WritableKeyPath<RecorderSettings, Bool>) -> some View {
        Toggle(
            "",
            isOn: Binding(
                get: { model.settings[keyPath: keyPath] },
                set: { value in model.update { $0[keyPath: keyPath] = value } }
            )
        )
        .toggleStyle(.switch)
        .labelsHidden()
    }
}

private struct FieldLabel: View {
    let text: String

    init(_ text: String) { self.text = text }

    var body: some View {
        Text(text)
            .font(.pixel(10))
            .foregroundStyle(.secondary)
            .gridColumnAlignment(.leading)
            .frame(width: 58, alignment: .leading)
    }
}
