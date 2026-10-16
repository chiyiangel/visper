#!/bin/sh
# Builds Visper.app from a release build and, with --dmg, a disk image to hand
# out. Output goes next to the binary, in <target dir>/[<triple>/]release/bundle.
#
#   scripts/bundle-macos.sh [--dmg] [--target <triple>]
#
# Signing is ad hoc, which only runs where it was built, unless
# VISPER_SIGN_IDENTITY names a "Developer ID Application" certificate in the
# keychain; then the app is signed with the hardened runtime. With --dmg and
# VISPER_NOTARY_PROFILE (a profile saved by `xcrun notarytool
# store-credentials`) as well, the disk image is notarized and stapled.
set -eu

cd "$(dirname "$0")/.."

dmg=false
target=""
while [ $# -gt 0 ]; do
    case "$1" in
        --dmg) dmg=true ;;
        --target)
            target="$2"
            shift
            ;;
        *)
            echo "usage: $0 [--dmg] [--target <triple>]" >&2
            exit 2
            ;;
    esac
    shift
done

target_dir=${CARGO_TARGET_DIR:-target}
if [ -n "$target" ]; then
    cargo build --release --target "$target"
    out="$target_dir/$target/release"
else
    cargo build --release
    out="$target_dir/release"
fi
binary="$out/visper"

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
# Grows with every commit, so a later build always reads as newer.
build=$(git rev-list --count HEAD 2>/dev/null || echo "$version")
# The oldest macOS the binary was linked for, so Finder turns older systems
# away instead of the app failing to start on them.
minimum=$(otool -l "$binary" | awk '/LC_BUILD_VERSION/ { found = 1 } found && $1 == "minos" { print $2; exit }')
arch=$(lipo -archs "$binary" | tr ' ' '-')

app="$out/bundle/Visper.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
sed -e "s/@VERSION@/$version/" \
    -e "s/@BUILD@/$build/" \
    -e "s/@MINIMUM_SYSTEM_VERSION@/$minimum/" \
    packaging/macos/Info.plist >"$app/Contents/Info.plist"
plutil -lint -s "$app/Contents/Info.plist"
printf 'APPL????' >"$app/Contents/PkgInfo"
cp "$binary" "$app/Contents/MacOS/visper"
cp assets/brand/AppIcon.icns "$app/Contents/Resources/"
cp -R packaging/macos/zh-Hans.lproj "$app/Contents/Resources/"

identity=${VISPER_SIGN_IDENTITY:--}
if [ "$identity" = "-" ]; then
    codesign --force --sign - "$app"
else
    codesign --force --sign "$identity" --options runtime --timestamp "$app"
fi
codesign --verify --strict "$app"
echo "$app"

if ! $dmg; then
    exit 0
fi

image="$out/bundle/Visper-$version-$arch.dmg"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
ditto "$app" "$stage/Visper.app"
# Dragging the app onto this link is the whole installation.
ln -s /Applications "$stage/Applications"
rm -f "$image"
hdiutil create -quiet -volname Visper -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$image"
if [ "$identity" != "-" ]; then
    codesign --force --sign "$identity" --timestamp "$image"
    if [ -n "${VISPER_NOTARY_PROFILE:-}" ]; then
        xcrun notarytool submit "$image" --keychain-profile "$VISPER_NOTARY_PROFILE" --wait
        xcrun stapler staple "$image"
    fi
fi
echo "$image"
