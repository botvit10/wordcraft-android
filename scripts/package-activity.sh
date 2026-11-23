#!/usr/bin/env bash
# Add the Android host class to cargo-apk's unsigned package, align it, then sign once.
set -euo pipefail
unsigned=$(realpath "${1:?unsigned cargo-apk output}")
output=$(realpath -m "${2:?destination APK}")
port=$(cd "$(dirname "$0")/.." && pwd)
sdk=${ANDROID_HOME:?}
bt="$sdk/build-tools/35.0.0"
platform="$sdk/platforms/android-35/android.jar"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/classes" "$work/dex"
javac -source 8 -target 8 -cp "$platform" -d "$work/classes" "$port/apps/wordcraft-android/java/WordCraftActivity.java"
jar cf "$work/activity.jar" -C "$work/classes" .
"$bt/d8" --min-api 26 --lib "$platform" --output "$work/dex" "$work/activity.jar"
cp "$unsigned" "$work/unsigned.apk"
(cd "$work/dex" && zip -q "$work/unsigned.apk" classes.dex)
"$bt/zipalign" -f 4 "$work/unsigned.apk" "$work/aligned.apk"
"$bt/apksigner" sign --ks "${CARGO_APK_RELEASE_KEYSTORE:?}" --ks-pass env:CARGO_APK_RELEASE_KEYSTORE_PASSWORD \
    --out "$work/signed.apk" "$work/aligned.apk"
"$bt/apksigner" verify "$work/signed.apk"
install -m 644 "$work/signed.apk" "$output"
