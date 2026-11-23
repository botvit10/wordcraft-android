#!/usr/bin/env bash
# Add the Android host class to cargo-apk's unsigned package, align it, then sign once.
set -euo pipefail
unsigned=$(realpath "${1:?unsigned cargo-apk output}")
output=$(realpath -m "${2:?destination APK}")
port=$(cd "$(dirname "$0")/.." && pwd)
sdk=${ANDROID_HOME:?}
if command -v cygpath >/dev/null; then sdk=$(cygpath -u "$sdk"); fi
bt="$sdk/build-tools/35.0.0"
platform="$sdk/platforms/android-35/android.jar"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/classes" "$work/dex"
javac -source 8 -target 8 -cp "$platform" -d "$work/classes" "$port/apps/wordcraft-android/java/WordCraftActivity.java"
jar cf "$work/activity.jar" -C "$work/classes" .
java -cp "$bt/lib/d8.jar" com.android.tools.r8.D8 --min-api 26 --lib "$platform" --output "$work/dex" "$work/activity.jar"
cp "$unsigned" "$work/unsigned.apk"
(cd "$work/dex" && zip -q "$work/unsigned.apk" classes.dex)
"$bt/zipalign" -f 4 "$work/unsigned.apk" "$work/aligned.apk"
java -jar "$bt/lib/apksigner.jar" sign --ks "${CARGO_APK_RELEASE_KEYSTORE:?}" --ks-pass env:CARGO_APK_RELEASE_KEYSTORE_PASSWORD \
    --out "$work/signed.apk" "$work/aligned.apk"
java -jar "$bt/lib/apksigner.jar" verify "$work/signed.apk"
install -m 644 "$work/signed.apk" "$output"
