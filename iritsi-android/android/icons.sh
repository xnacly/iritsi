#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

launcher=assets/icons/256x256.png
notification=assets/icons/notification.png
res=android/res

for density in mdpi:48:24 hdpi:72:36 xhdpi:96:48 xxhdpi:144:72 xxxhdpi:192:96; do
    IFS=: read -r name launcher_size notification_size <<< "$density"
    mkdir -p "$res/mipmap-$name" "$res/drawable-$name"
    magick "$launcher" -filter point -resize "${launcher_size}x${launcher_size}" \
        "$res/mipmap-$name/ic_launcher.png"
    if [ -f "$notification" ]; then
        magick "$notification" -fill white -colorize 100 -filter point \
            -resize "${notification_size}x${notification_size}" "$res/drawable-$name/ic_notification.png"
    else
        magick "$launcher" -fuzz 12% -transparent white -fill white -colorize 100 -filter point \
            -resize "${notification_size}x${notification_size}" "$res/drawable-$name/ic_notification.png"
    fi
done
