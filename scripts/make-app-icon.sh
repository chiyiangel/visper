#!/bin/sh
# Renders assets/brand/*.svg into AppIcon.icns and the PNG the app shows in the
# Dock when it runs unbundled. Needs rsvg-convert (brew install librsvg).
set -eu

cd "$(dirname "$0")/../assets/brand"
export PATH="/opt/homebrew/bin:$PATH"

iconset=AppIcon.iconset
rm -rf "$iconset"
mkdir "$iconset"

# At 16 and 32 px the two thin lines blur, so those sizes use the merged bar.
render() { # source size name
    rsvg-convert -w "$2" -h "$2" "$1" -o "$iconset/icon_$3.png"
}
render app-icon-small.svg 16 16x16
render app-icon-small.svg 32 16x16@2x
render app-icon-small.svg 32 32x32
render app-icon.svg 64 32x32@2x
render app-icon.svg 128 128x128
render app-icon.svg 256 128x128@2x
render app-icon.svg 256 256x256
render app-icon.svg 512 256x256@2x
render app-icon.svg 512 512x512
render app-icon.svg 1024 512x512@2x

iconutil -c icns "$iconset" -o AppIcon.icns
rm -rf "$iconset"

# Embedded by src/dock.rs.
rsvg-convert -w 512 -h 512 app-icon.svg -o app-icon-512.png
