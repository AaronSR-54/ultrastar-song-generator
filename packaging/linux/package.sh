#!/usr/bin/env bash
# Builds the Debian package for the desktop app.
#
# The script only packages an existing release binary: it does not compile.
#
# Usage:
#   packaging/linux/package.sh
#
# Environment:
#   BINARY   release binary to package (default: target/release/usdx-desktop)
#   OUT_DIR  output directory (default: dist/)
#   DEB_ARCH Debian architecture (default: dpkg --print-architecture, or amd64)
#
# Requires dpkg-deb and dpkg-shlibdeps (package `dpkg-dev`), because the
# dependency list has to match the system that links the binary.
set -euo pipefail

pkg=ultrastar-song-generator
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
src="$root/packaging/linux"

die() {
  echo "error: $*" >&2
  exit 1
}

command -v dpkg-deb >/dev/null || die "dpkg-deb not found (install dpkg)"
command -v dpkg-shlibdeps >/dev/null || die "dpkg-shlibdeps not found (install dpkg-dev)"

# The workspace manifest is the only source of truth for the version.
eval "$("$root/packaging/version.sh")"

binary="${BINARY:-$root/target/release/usdx-desktop}"
out_dir="${OUT_DIR:-$root/dist}"
[[ -f "$binary" ]] || die "binary not found: $binary (run: cargo build --release -p usdx-desktop)"

if [[ -n "${DEB_ARCH:-}" ]]; then
  arch="$DEB_ARCH"
elif command -v dpkg >/dev/null; then
  arch="$(dpkg --print-architecture)"
else
  arch=amd64
fi

if command -v desktop-file-validate >/dev/null; then
  desktop-file-validate "$src/$pkg.desktop"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
stage="$work/root"

mkdir -p "$stage/usr/lib/$pkg" "$stage/usr/bin" "$stage/usr/share/doc/$pkg"

# Binary lives in a private directory and is reached through /usr/bin so the
# usual `usdx-desktop` command works and the executable is not in PATH twice.
install -Dm755 "$binary" "$stage/usr/lib/$pkg/usdx-desktop"
ln -s "../lib/$pkg/usdx-desktop" "$stage/usr/bin/usdx-desktop"

install -Dm644 "$src/$pkg.desktop" "$stage/usr/share/applications/$pkg.desktop"

shopt -s nullglob
icons=("$root"/assets/icons/hicolor/*/apps/"$pkg".png)
shopt -u nullglob
[[ ${#icons[@]} -gt 0 ]] || die "no icons found in assets/icons/hicolor"
for icon in "${icons[@]}"; do
  size="$(basename "$(dirname "$(dirname "$icon")")")"
  install -Dm644 "$icon" "$stage/usr/share/icons/hicolor/$size/apps/$pkg.png"
done

install -Dm644 "$src/copyright" "$stage/usr/share/doc/$pkg/copyright"
{
  printf '%s (%s) unstable; urgency=medium\n\n' "$pkg" "$VERSION_DISPLAY"
  printf '  * Release %s.\n\n' "$VERSION_DISPLAY"
  printf ' -- Aaron Sanz Roca <a.sanzblog6b@gmail.com>  %s\n' "$(date -R)"
} | gzip -9n >"$stage/usr/share/doc/$pkg/changelog.gz"
chmod 644 "$stage/usr/share/doc/$pkg/changelog.gz"

# dpkg-shlibdeps resolves the libraries the binary links against (libasound,
# libstdc++, libgcc-s1, libc6) with the exact package names of this distro, so
# they are never hardcoded here.
shlibs_dir="$work/shlibs"
mkdir -p "$shlibs_dir/debian"
cat >"$shlibs_dir/debian/control" <<EOF
Source: $pkg
Section: sound
Priority: optional
Maintainer: Aaron Sanz Roca <a.sanzblog6b@gmail.com>

Package: $pkg
Architecture: any
Depends: \${misc:Depends}
Description: dependency resolution helper
EOF

shlibs_out="$(cd "$shlibs_dir" && dpkg-shlibdeps -O "$stage/usr/lib/$pkg/usdx-desktop")"
shlibs_depends="${shlibs_out#shlibs:Depends=}"
[[ -n "$shlibs_depends" ]] || die "dpkg-shlibdeps produced no dependencies"

# winit and rfd open these with dlopen, so neither `ldd` nor dpkg-shlibdeps
# sees them. Missing one is a window that never appears.
dlopen_depends=(
  libxkbcommon0
  libxkbcommon-x11-0
  libwayland-client0
  libx11-6
  libxcb1
  libxcursor1
  libxrandr2
  libxi6
)

declare -A seen=()
depends=()
add_dep() {
  local spec="$1" name
  name="${spec%% *}"
  [[ -n "$name" ]] || return 0
  [[ -n "${seen[$name]:-}" ]] && return 0
  seen["$name"]=1
  depends+=("$spec")
}

IFS=',' read -r -a linked <<<"$shlibs_depends"
for dep in "${linked[@]}"; do
  # dpkg-shlibdeps separates entries with ", " and pads with spaces.
  dep="${dep#"${dep%%[![:space:]]*}"}"
  dep="${dep%"${dep##*[![:space:]]}"}"
  add_dep "$dep"
done
for dep in "${dlopen_depends[@]}"; do
  add_dep "$dep"
done

depends_line="$(printf '%s, ' "${depends[@]}")"
depends_line="${depends_line%, }"

installed_size="$(du -k -s "$stage" | cut -f1)"

mkdir -p "$stage/DEBIAN"
sed -e "s|@@VERSION@@|$VERSION_DISPLAY|" \
  -e "s|@@ARCH@@|$arch|" \
  -e "s|@@INSTALLED_SIZE@@|$installed_size|" \
  -e "s|@@DEPENDS@@|$depends_line|" \
  "$src/control.in" >"$stage/DEBIAN/control"

# `dpkg-deb --build` only adds md5sums by itself on newer releases; write them
# here so the package is identical no matter which dpkg built it.
(
  cd "$stage"
  find . -type f ! -path './DEBIAN/*' -printf '%P\0' |
    LC_ALL=C sort -z |
    xargs -0 --no-run-if-empty md5sum
) >"$stage/DEBIAN/md5sums"
chmod 644 "$stage/DEBIAN/md5sums"

mkdir -p "$out_dir"
artifact="$out_dir/${pkg}_${VERSION_DISPLAY}_${arch}.deb"
dpkg-deb --build --root-owner-group "$stage" "$artifact"

echo "$artifact"
