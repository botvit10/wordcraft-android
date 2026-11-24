#!/usr/bin/env bash
# Run the platform bridge on a disposable API 35 emulator. The instrumentation and the
# target must carry the same test signature. Pass the signed app and a new work directory.
set -euo pipefail
port=$(cd "$(dirname "$0")/.." && pwd)
apk=$(realpath "$1")
mkdir -p "$2"
cd "$2"
bt="$ANDROID_HOME/build-tools/35.0.0"
platform="$ANDROID_HOME/platforms/android-35/android.jar"
mkdir -p classes dex
javac -source 8 -target 8 -cp "$platform" -d classes "$port"/tests/android/*.java
jar cf tests.jar -C classes .
"$bt/d8" --lib "$platform" --min-api 26 --output dex tests.jar
"$bt/aapt2" compile --dir "$port/tests/android/res" -o res.zip
"$bt/aapt2" link res.zip -o test.apk -I "$platform" --min-sdk-version 26 --target-sdk-version 35 --manifest "$port/tests/android/AndroidManifest.xml"
(cd dex && zip -q ../test.apk classes.dex)
"$bt/apksigner" sign --ks "$CARGO_APK_RELEASE_KEYSTORE" --ks-pass env:CARGO_APK_RELEASE_KEYSTORE_PASSWORD test.apk
adb install --no-incremental -r "$apk"
adb install --no-incremental -r -t test.apk
keyboard=ai.storyteller.wordcraft.test/.TestKeyboard
# InputMethodManager refreshes its service list asynchronously after package installation.
timeout 30 bash -c 'until adb shell ime list -s -a | tr -d "\r" | grep -Fxq "$1"; do sleep 1; done' _ "$keyboard"
adb shell ime enable "$keyboard"
adb shell ime set "$keyboard"
adb shell settings put secure show_ime_with_hard_keyboard 1
failed=0
for probe in keyboard picker; do
    adb shell am force-stop ai.storyteller.wordcraft
    if ! timeout 150 adb shell am instrument -w -e probe "$probe" ai.storyteller.wordcraft.test/.DeviceTest | tee "$probe.txt" ||
       ! grep -Fxq 'INSTRUMENTATION_CODE: -1' "$probe.txt"; then
        failed=1
        adb logcat -d -t 250 '*:E'
    fi
done
exit "$failed"
