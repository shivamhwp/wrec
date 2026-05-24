#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

APP_NAME="${APP_NAME:-Wrec}"
BIN_NAME="${BIN_NAME:-wrec}"
BUNDLE_ID="${BUNDLE_ID:-app.wrec.wrec}"
CODESIGN_IDENTITY="${CODESIGN_IDENTITY:--}"
NOTARIZE="${NOTARIZE:-0}"
CREATE_DMG="${CREATE_DMG:-1}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
DIST_DIR="$ROOT/dist/macos"
APP="$DIST_DIR/$APP_NAME.app"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"
INFO_PLIST="$CONTENTS/Info.plist"
ENTITLEMENTS="$ROOT/packaging/macos/entitlements.plist"
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/crates/app/Cargo.toml" | head -n 1)}"

if [[ "$NOTARIZE" == "1" && "$CODESIGN_IDENTITY" == "-" ]]; then
  echo "NOTARIZE=1 requires CODESIGN_IDENTITY to be a Developer ID Application identity" >&2
  exit 1
fi

cargo build --release -p wrec-app --bin "$BIN_NAME"

HELPER="$(find "$TARGET_DIR/release/build" -path "*/out/wrec-helper" -type f -print | sort | tail -n 1)"
if [[ -z "$HELPER" ]]; then
  echo "Could not find compiled wrec-helper in $TARGET_DIR/release/build" >&2
  exit 1
fi

rm -rf "$APP"
mkdir -p "$MACOS" "$RESOURCES"

cp "$TARGET_DIR/release/$BIN_NAME" "$MACOS/$BIN_NAME"
cp "$HELPER" "$MACOS/wrec-helper"
cp "$ROOT/packaging/macos/Info.plist" "$INFO_PLIST"

if [[ -f "$ROOT/packaging/macos/AppIcon.icns" ]]; then
  cp "$ROOT/packaging/macos/AppIcon.icns" "$RESOURCES/AppIcon.icns"
  /usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string AppIcon" "$INFO_PLIST" 2>/dev/null \
    || /usr/libexec/PlistBuddy -c "Set :CFBundleIconFile AppIcon" "$INFO_PLIST"
fi

/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $BUNDLE_ID" "$INFO_PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$INFO_PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" "$INFO_PLIST"

sign_args=(--force --options runtime --sign "$CODESIGN_IDENTITY")
if [[ "$CODESIGN_IDENTITY" != "-" ]]; then
  sign_args+=(--timestamp)
fi

codesign "${sign_args[@]}" "$MACOS/wrec-helper"
codesign "${sign_args[@]}" --entitlements "$ENTITLEMENTS" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

if [[ "$CREATE_DMG" == "1" ]]; then
  DMG="$DIST_DIR/$APP_NAME-$VERSION.dmg"
  rm -f "$DMG"
  hdiutil create -volname "$APP_NAME" -srcfolder "$APP" -ov -format UDZO "$DMG"
  echo "Created $DMG"

  if [[ "$NOTARIZE" == "1" ]]; then
    : "${APPLE_ID:?APPLE_ID is required for notarization}"
    : "${APPLE_TEAM_ID:?APPLE_TEAM_ID is required for notarization}"
    : "${APPLE_APP_PASSWORD:?APPLE_APP_PASSWORD is required for notarization}"

    xcrun notarytool submit "$DMG" \
      --apple-id "$APPLE_ID" \
      --team-id "$APPLE_TEAM_ID" \
      --password "$APPLE_APP_PASSWORD" \
      --wait
    xcrun stapler staple "$DMG"
    spctl -a -vv -t open --context context:primary-signature "$DMG"
  fi
else
  echo "Created $APP"
fi
