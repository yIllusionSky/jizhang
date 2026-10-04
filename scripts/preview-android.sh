#!/usr/bin/env bash
set -euo pipefail
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
if [[ $# -eq 0 ]]; then
    echo '用法: scripts/preview-android.sh <AVD名称> [模拟器参数]'
    "$ANDROID_HOME/emulator/emulator" -list-avds
    exit 0
fi
avd_name="$1"
shift
# Lavapipe avoids host Vulkan/SwiftShader shader-driver failures on Apple Silicon.
exec "$ANDROID_HOME/emulator/emulator" -avd "$avd_name" -gpu lavapipe -no-snapshot-load -no-snapshot-save "$@"
