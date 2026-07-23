#!/usr/bin/env bash
set -euo pipefail

# Runs the real updater end to end in a disposable directory: packages an old
# release-like app and a newer replacement, archives the replacement, launches
# the old app with local release metadata, then verifies the atomic swap.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-9.9.9}"
PREVIEW_DIR="$ROOT/dist/update-preview"
APP_NAME="Wrec Update Preview"
APP="$PREVIEW_DIR/installed/$APP_NAME.app"
ARCHIVE="$PREVIEW_DIR/wrec-app-update.tar.gz"
DATA_DIR="$PREVIEW_DIR/data"
HOME_DIR="$PREVIEW_DIR/home"

log() {
  printf '[wrec-preview-update] %s\n' "$*"
}

if [ "${1:-}" = "clean" ]; then
  rm -rf "$PREVIEW_DIR"
  log "Update preview artifacts removed."
  exit 0
fi

rm -rf "$PREVIEW_DIR"
mkdir -p "$PREVIEW_DIR/installed" "$DATA_DIR" "$HOME_DIR"

log "Packaging installed version 0.0.1"
APP_NAME="$APP_NAME" BUNDLE_ID="app.wrec.update-preview" VERSION="0.0.1" \
  CREATE_DMG=0 "$ROOT/scripts/package-macos.sh" release
ditto "$ROOT/dist/release/$APP_NAME.app" "$APP"

log "Packaging replacement version $VERSION"
APP_NAME="$APP_NAME" BUNDLE_ID="app.wrec.update-preview" VERSION="$VERSION" \
  CREATE_DMG=0 "$ROOT/scripts/package-macos.sh" release
tar -C "$ROOT/dist/release" -czf "$ARCHIVE" "$APP_NAME.app"

log "Running updater smoke"
WREC_DATA_DIR="$DATA_DIR" \
WREC_HOME="$HOME_DIR" \
WREC_UPDATE_VERSION="$VERSION" \
WREC_UPDATE_ARCHIVE="$ARCHIVE" \
WREC_UPDATE_SMOKE=1 \
  "$APP/Contents/MacOS/wrec-app"

actual="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
if [ "$actual" != "$VERSION" ]; then
  printf '[wrec-preview-update] error: expected %s, got %s\n' "$VERSION" "$actual" >&2
  exit 1
fi
codesign --verify --deep --strict "$APP"
log "PASS: installed bundle is now $VERSION"
