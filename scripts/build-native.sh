#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
case "$(uname -s)" in
    Darwin) ndk_host=darwin-x86_64; default_sdk="$HOME/Library/Android/sdk" ;;
    Linux) ndk_host=linux-x86_64; default_sdk="$HOME/Android/Sdk" ;;
    *) echo 'Unsupported build host; use macOS or Linux.' >&2; exit 1 ;;
esac
export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$default_sdk}}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.0.12077973}"
toolchain="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$ndk_host/bin"
export CC_aarch64_linux_android="$toolchain/aarch64-linux-android26-clang"
export CXX_aarch64_linux_android="$toolchain/aarch64-linux-android26-clang++"
export AR_aarch64_linux_android="$toolchain/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"
profile="${1:-debug}"
case "$profile" in
    release) cargo build --locked -p jizhang --target aarch64-linux-android --release ;;
    debug) cargo build --locked -p jizhang --target aarch64-linux-android ;;
    *) echo 'Expected debug or release profile.' >&2; exit 1 ;;
esac
output="android/app/build/rustJniLibs/$profile/arm64-v8a"
mkdir -p "$output"
cp "target/aarch64-linux-android/$profile/libjizhang.so" "$output/"
