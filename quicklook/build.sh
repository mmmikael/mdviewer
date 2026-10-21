#!/usr/bin/env bash
# Builds the Quick Look preview extension, MDViewerQuickLook.appex, which
# `tauri build` then copies into MDViewer.app/Contents/PlugIns (see
# bundle.macOS.files in src-tauri/tauri.conf.json). Runs as Tauri's
# beforeBundleCommand; it can also be run on its own.
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=src-tauri/target/quicklook
APPEX="$OUT/MDViewerQuickLook.appex"
VERSION=$(node -p "require('./src-tauri/tauri.conf.json').version")

# Always universal: it's small, and doesn't depend on which target Tauri builds.
ARCHS="arm64 x86_64"

rm -rf "$APPEX"
mkdir -p "$APPEX/Contents/MacOS" "$APPEX/Contents/Resources"

echo "Quick Look: building the preview page"
pnpm exec vite build --config quicklook/vite.config.ts --outDir "$APPEX/Contents/Resources/web" --logLevel warn
cp quicklook/preview.html "$APPEX/Contents/Resources/web/"
# Mermaid's single-file build, loaded by the page only for documents with diagrams.
cp node_modules/mermaid/dist/mermaid.min.js "$APPEX/Contents/Resources/web/"

echo "Quick Look: compiling the extension for $ARCHS"
binaries=()
for arch in $ARCHS; do
  swiftc -O -target "$arch-apple-macos13.0" -application-extension -parse-as-library \
    -module-name MDViewerQuickLook -Xlinker -e -Xlinker _NSExtensionMain \
    -framework Cocoa -framework Quartz -framework WebKit \
    quicklook/PreviewViewController.swift -o "$OUT/MDViewerQuickLook-$arch"
  binaries+=("$OUT/MDViewerQuickLook-$arch")
done
lipo -create "${binaries[@]}" -output "$APPEX/Contents/MacOS/MDViewerQuickLook"
sed "s/@VERSION@/$VERSION/g" quicklook/Info.plist > "$APPEX/Contents/Info.plist"

# Same identity Tauri uses for the app ("-" means ad-hoc). Extensions must be
# signed (they're sandboxed), and before the app around them.
IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
flags=()
if [ "$IDENTITY" != "-" ]; then flags=(--options runtime --timestamp); fi
codesign --force --sign "$IDENTITY" ${flags[@]+"${flags[@]}"} \
  --entitlements quicklook/entitlements.plist "$APPEX"
echo "Quick Look: built $APPEX"
