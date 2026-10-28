#!/bin/sh
# Packs the release build into an AppImage: packaging/linux/appimage.sh <version>
set -eu
version="$1"
dir=target/appimage/DystoPad.AppDir
rm -rf "$dir"
mkdir -p "$dir/usr/bin"
cp target/release/dystopad "$dir/usr/bin/"
cp packaging/linux/dystopad.desktop "$dir/"
cp assets/icon/icon.png "$dir/dystopad.png"
ln -s usr/bin/dystopad "$dir/AppRun"
if [ ! -x target/appimage/appimagetool ]; then
  curl -fsSL -o target/appimage/appimagetool https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x target/appimage/appimagetool
fi
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 target/appimage/appimagetool "$dir" "target/release/DystoPad_${version}_amd64.AppImage"
