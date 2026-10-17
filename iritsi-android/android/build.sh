#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

target="${1:-aarch64-linux-android}"
project=../target/dx/iritsi/debug/android/app
res="$project/app/src/main/res"

rm -f "$res"/mipmap-*/ic_launcher.png "$res"/mipmap-*/iritsi_*.png "$res"/drawable-*/ic_notification.png
dx build --platform android --target "$target"
rm -f "$res"/mipmap-*/ic_launcher.webp "$res"/mipmap-anydpi-v26/ic_launcher.xml
cp -r android/res/. "$res"/
(cd "$project" && ./gradlew assembleDebug --quiet)
echo "$project/app/build/outputs/apk/debug/app-debug.apk"
