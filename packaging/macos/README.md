# macOS packaging

`scripts/package-macos.sh` assembles the direct-distribution app bundle:

```text
Wrec.app/
  Contents/
    Info.plist
    MacOS/
      wrec
      wrec-helper
    Resources/
```

The packaged app resolves `wrec-helper` beside the `wrec` executable at runtime.
Cargo development still falls back to the helper path emitted by
`crates/macos/build.rs`.

For local ad-hoc packaging:

```bash
./scripts/package-macos.sh
```

For Developer ID signing:

```bash
CODESIGN_IDENTITY="Developer ID Application: Example, Inc. (TEAMID)" \
./scripts/package-macos.sh
```

For notarization, provide App Store Connect credentials and enable notarization:

```bash
CODESIGN_IDENTITY="Developer ID Application: Example, Inc. (TEAMID)" \
APPLE_ID="dev@example.com" \
APPLE_TEAM_ID="TEAMID" \
APPLE_APP_PASSWORD="app-specific-password" \
NOTARIZE=1 \
./scripts/package-macos.sh
```

Add `packaging/macos/AppIcon.icns` when the icon is ready; the script copies it
automatically if present.
