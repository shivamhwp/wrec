// config.json persistence, wire-compatible with the Rust `config` crate
// (snake_case keys, pretty-printed). Writes are debounced-by-nature: the model
// saves on discrete user actions only, never on a timer.

import Foundation

enum ConfigStore {
    private static let encoder: JSONEncoder = {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return encoder
    }()

    private static let decoder: JSONDecoder = {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return decoder
    }()

    static func load() -> AppConfig {
        guard let data = try? Data(contentsOf: WrecPaths.configPath()),
            var config = try? decoder.decode(AppConfig.self, from: data)
        else {
            return AppConfig(
                settings: .defaults(),
                selectedTargetKey: nil,
                showNerdLogs: false
            )
        }
        config.settings.applyPresetLimits()
        return config
    }

    static func save(_ config: AppConfig) {
        do {
            let dir = WrecPaths.dataDir()
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            let data = try encoder.encode(config)
            try data.write(to: WrecPaths.configPath(), options: .atomic)
        } catch {
            NSLog("wrec: config save failed: \(error)")
        }
    }
}
