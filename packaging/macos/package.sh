#!/usr/bin/env bash
# Builds the macOS disk image (.dmg) for the desktop app.
#
# Only an existing release binary is packaged; nothing is compiled here. The
# .app is unsigned: Gatekeeper needs the documented right-click → Open path.
#
# Usage:
#   packaging/macos/package.sh
#
# Environment:
#   BINARY   release binary to package (default: target/release/usdx-desktop)
#   OUT_DIR  output directory (default: dist/)
#
# Requires macOS tools: iconutil, plutil, hdiutil.
set -euo pipefail

app_name="UltraStar Song Generator"
bundle_id="com.aaronsr.ultrastar-song-generator"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
src="$root/packaging/macos"

die() {
  echo "error: $*" >&2
  exit 1
}

for tool in iconutil plutil hdiutil; do
  command -v "$tool" >/dev/null || die "$tool not found: this script must run on macOS"
done

# The workspace manifest is the only source of truth for the version.
eval "$("$root/packaging/version.sh")"

binary="${BINARY:-$root/target/release/usdx-desktop}"
out_dir="${OUT_DIR:-$root/dist}"
[[ -f "$binary" ]] || die "binary not found: $binary (run: cargo build --release -p usdx-desktop)"

if command -v lipo >/dev/null; then
  lipo -archs "$binary" | tr ' ' '\n' | grep -qx arm64 ||
    die "binary is not arm64: $(lipo -archs "$binary")"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

app="$work/$app_name.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

install -m 755 "$binary" "$app/Contents/MacOS/usdx-desktop"
printf 'APPL????' >"$app/Contents/PkgInfo"

# iconutil needs the complete iconset. The repository commits every size up to
# 512 px, so the two largest entries are derived from the committed assets.
iconset="$work/AppIcon.iconset"
mkdir -p "$iconset"
cp "$root"/assets/icons/AppIcon.iconset/*.png "$iconset/"
cp "$iconset/icon_256x256@2x.png" "$iconset/icon_512x512.png"
cp "$root/assets/icon.png" "$iconset/icon_512x512@2x.png"
iconutil -c icns -o "$app/Contents/Resources/AppIcon.icns" "$iconset"

sed -e "s|@@VERSION_DISPLAY@@|$VERSION_DISPLAY|" \
  -e "s|@@VERSION_NUMERIC@@|$VERSION_NUMERIC|" \
  "$src/Info.plist" >"$app/Contents/Info.plist"

plutil -lint "$app/Contents/Info.plist"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Contents/Info.plist")" == "$bundle_id" ]] ||
  die "unexpected bundle identifier"

# Staging folder for hdiutil: the app plus the usual drag-to-install symlink.
stage="$work/dmg"
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"

mkdir -p "$out_dir"
artifact="$out_dir/UltraStar-Song-Generator-${VERSION_DISPLAY}-macos-arm64.dmg"
hdiutil create -volname "$app_name" -srcfolder "$stage" -ov -format UDZO "$artifact"

echo "$artifact"
