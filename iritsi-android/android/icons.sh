#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

source=../icon/no-bg256x256.png
res=android/res
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

magick "$source" -filter point -resize 64x64 -trim +repage "$work/rings.png"
magick "$source" -filter point -resize 64x64 -channel A -threshold 50% +channel \
    -fill white -colorize 100 -trim +repage "$work/mask.png"

rm -rf "$res"
for density in mdpi:1 hdpi:1.5 xhdpi:2 xxhdpi:3 xxxhdpi:4; do
    IFS=: read -r name scale <<< "$density"
    canvas=$(awk "BEGIN { print 108 * $scale }")
    art=$(awk "BEGIN { print 100 * $scale }")
    notification=$(awk "BEGIN { print 24 * $scale }")
    inner=$(awk "BEGIN { print 20 * $scale }")
    mkdir -p "$res/mipmap-$name" "$res/drawable-$name"
    for layer in rings:iritsi_foreground mask:iritsi_monochrome; do
        IFS=: read -r input output <<< "$layer"
        magick "$work/$input.png" -filter point -resize "$art%" -background none \
            -gravity center -extent "${canvas}x${canvas}" "$res/mipmap-$name/$output.png"
    done
    magick "$work/mask.png" -filter point -resize "${inner}x${inner}" -background none \
        -gravity center -extent "${notification}x${notification}" "$res/drawable-$name/ic_notification.png"
done

mkdir -p "$res/mipmap-anydpi-v26"
cat > "$res/mipmap-anydpi-v26/ic_launcher.xml" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@android:color/white" />
    <foreground android:drawable="@mipmap/iritsi_foreground" />
    <monochrome android:drawable="@mipmap/iritsi_monochrome" />
</adaptive-icon>
XML
