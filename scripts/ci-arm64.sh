#!/usr/bin/env bash
# Linux x86_64. Pass a new directory outside the checkout.
set -euo pipefail
port=$(cd "$(dirname "$0")/.." && pwd)
work=$(realpath -m "${1:?pass a new build directory}")
mkdir "$work"
pin=2ffba93b5189410a415970be179dcf481092d756
git init -q "$work/source"
cd "$work/source"
git fetch --depth 1 https://github.com/storytold/wordcraft.git "$pin"
git checkout -q --detach FETCH_HEAD
test "$(git rev-parse HEAD)" = "$pin"
git apply "$port/patches/0001-wordcraft-android-port.patch" "$port/patches/0002-android-lock.patch"
cp -r "$port/apps/wordcraft-android" apps/
cp -r "$port/vendor" .

sdk=${ANDROID_HOME:?set ANDROID_HOME to the Android SDK}
export ANDROID_NDK_ROOT="$sdk/ndk/27.2.12479018"
bin="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$bin/aarch64-linux-android26-clang"
export CC_aarch64_linux_android="$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
export CXX_aarch64_linux_android="$bin/aarch64-linux-android26-clang++"
export AR_aarch64_linux_android="$bin/llvm-ar"
# cargo-apk chooses the highest installed build-tools. Expose only the selected version.
export ANDROID_HOME="$work/sdk" ANDROID_SDK_ROOT="$work/sdk"
mkdir -p "$ANDROID_HOME/build-tools" "$ANDROID_HOME/platforms"
bt="$sdk/build-tools/35.0.0"
ln -s "$bt" "$ANDROID_HOME/build-tools/35.0.0"
ln -s "$sdk/platforms/android-35" "$ANDROID_HOME/platforms/android-35"
ln -s "$sdk/platform-tools" "$ANDROID_HOME/platform-tools"
export RUSTUP_TOOLCHAIN=1.99.0
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
cargo fetch --locked --target aarch64-linux-android --target x86_64-unknown-linux-gnu
cargo test --locked -p wordcraft-android --features eframe/x11
lock_hash=$(sha256sum Cargo.lock)
# This key signs only the current test build. Only the artifact directory is uploaded.
umask 077
export CARGO_APK_RELEASE_KEYSTORE="$work/test.keystore"
export CARGO_APK_RELEASE_KEYSTORE_PASSWORD=android
keytool -genkeypair -keystore "$CARGO_APK_RELEASE_KEYSTORE" -alias wordcraft-test \
  -keyalg RSA -keysize 2048 -validity 365 -storepass android -keypass android \
  -dname 'CN=WordCraft Development Test'
CARGO_NET_OFFLINE=true cargo apk build --release -p wordcraft-android --target aarch64-linux-android
printf '%s\n' "$lock_hash" | sha256sum --check
out="$work/artifact"
mkdir -p "$out/licenses"
apk="$out/wordcraft-arm64-test.apk"
"$port/scripts/package-activity.sh" "$CARGO_TARGET_DIR/release/apk/wordcraft-unaligned.apk" "$apk"
"$bt/apksigner" verify --verbose "$apk"
"$bt/zipalign" -c 4 "$apk"
badging=$("$bt/aapt2" dump badging "$apk")
grep -F "package: name='ai.storyteller.wordcraft'" <<< "$badging"
grep -Fx "native-code: 'arm64-v8a'" <<< "$badging"
grep -F "launchable-activity: name='ai.storyteller.wordcraft.WordCraftActivity'" <<< "$badging"
unzip -p "$apk" classes.dex > "$work/classes.dex"
"$bt/dexdump" "$work/classes.dex" > "$work/dex.txt"
grep -F "Lai/storyteller/wordcraft/WordCraftActivity;" "$work/dex.txt"
unzip -p "$apk" lib/arm64-v8a/libwordcraft.so > "$work/libwordcraft.so"
symbols=$("$bin/llvm-nm" --dynamic --defined-only "$work/libwordcraft.so")
grep -E ' T android_main$' <<< "$symbols"
grep -E ' T ANativeActivity_onCreate$' <<< "$symbols"
cp NOTICE LICENSE-MIT LICENSE-APACHE ATTRIBUTION.md assets/fonts/OFL-*.txt "$out/licenses/"
cp "$port/ATTRIBUTION.md" "$out/licenses/ATTRIBUTION-android.md"
cp vendor/winit/LICENSE "$out/licenses/LICENSE-winit"
python3 "$port/scripts/collect-notices.py" "$out/licenses"
cd "$out"
sha256sum wordcraft-arm64-test.apk > SHA256SUMS
