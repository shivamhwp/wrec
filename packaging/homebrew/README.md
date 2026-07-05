# Homebrew packaging

These files are the source of truth for the Homebrew tap. They ship binaries
from GitHub Releases, so the tap repo (`shivamhwp/homebrew-tap`) just mirrors
this directory:

```text
homebrew-tap/
  Casks/wrec.rb        # the app: brew install --cask shivamhwp/tap/wrec
  Formula/wrec-cli.rb  # the CLI:  brew install shivamhwp/tap/wrec-cli
```

After publishing a `v*` GitHub release, refresh the version and checksums and
copy the result into the tap repo:

```bash
./scripts/update-homebrew.sh 0.1.0
```

Notes:

- Builds are not notarized, so `brew install --cask wrec` quarantines the app
  and macOS blocks the first launch with an "Apple could not verify" dialog
  (Homebrew removed `--no-quarantine` in v4.3, so there is no install-time
  opt-out anymore). Clear it recursively — the nested `daemon` and
  `capture-engine` helpers are quarantined separately, so a non-recursive
  removal still gets blocked on the second launch:

  ```bash
  xattr -dr com.apple.quarantine /Applications/Wrec.app
  ```

  Alternatively, launch once, dismiss the dialog, then approve it in
  System Settings > Privacy & Security > "Open Anyway".
- The CLI formula is not affected: Homebrew-downloaded formula binaries do not
  get the quarantine attribute.
- Only Apple Silicon artifacts are published, so both are `arch: :arm64`.
