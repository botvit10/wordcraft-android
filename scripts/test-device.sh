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
javac -source 8 -target 8 -cp "$platform" -d classes "$port/tests/android/DeviceTest.java"
jar cf tests.jar -C classes .
"$bt/d8" --lib "$platform" --min-api 26 --output dex tests.jar
"$bt/aapt2" link -o test.apk -I "$platform" --min-sdk-version 26 --target-sdk-version 35 --manifest "$port/tests/android/AndroidManifest.xml"
(cd dex && zip -q ../test.apk classes.dex)
"$bt/apksigner" sign --ks "$CARGO_APK_RELEASE_KEYSTORE" --ks-pass env:CARGO_APK_RELEASE_KEYSTORE_PASSWORD test.apk
adb install -r "$apk"
adb install -r -t test.apk
adb shell am instrument -w ai.storyteller.wordcraft.test/.DeviceTest | tee result.txt
grep -Fx 'INSTRUMENTATION_CODE: -1' result.txt
