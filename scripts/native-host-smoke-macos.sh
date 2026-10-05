#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "native-host-smoke-macos.sh must run on macOS" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${ARTIFACT_DIR:-"$ROOT_DIR/target/native-host-smoke/macos"}"
AUTO_SMOKE="${ZSCLIP_NATIVE_HOST_AUTO_SMOKE:-1}"
SHELL_OPEN_DRY_RUN="${ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN:-1}"
if [[ "$SHELL_OPEN_DRY_RUN" == "1" ]]; then
  SHELL_OPEN_DRY_RUN_LOG=true
else
  SHELL_OPEN_DRY_RUN_LOG=false
fi
APP_LOG="$ARTIFACT_DIR/zsclip-appkit.log"
SCREENSHOT="$ARTIFACT_DIR/zsclip-appkit-main.png"

mkdir -p "$ARTIFACT_DIR"
cd "$ROOT_DIR"

echo "==> macOS AppKit native host tests"
cargo test -q macos_native_host_launch_plan_targets_real_appkit_entry
cargo test -q macos_native_host_actions_enter_product_command_routes
cargo test -q macos_native_row_actions_enter_product_command_routes
cargo test -q macos_native_status_menu_actions_enter_product_command_routes
cargo test -q macos_native_settings_control_actions_enter_product_command_routes
cargo test -q macos_native_vv_select_enters_product_event_bridge

echo "==> macOS AppKit build"
cargo build -q --bin zsclip

echo "==> Launching ZSClip AppKit host"
ZSCLIP_NATIVE_HOST_AUTO_SMOKE="$AUTO_SMOKE" ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN="$SHELL_OPEN_DRY_RUN" "$ROOT_DIR/target/debug/zsclip" >"$APP_LOG" 2>&1 &
APP_PID=$!

cleanup() {
  if kill -0 "$APP_PID" >/dev/null 2>&1; then
    kill "$APP_PID" >/dev/null 2>&1 || true
    wait "$APP_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

sleep "${NATIVE_HOST_SMOKE_WAIT:-3}"
if ! kill -0 "$APP_PID" >/dev/null 2>&1; then
  echo "ZSClip exited before screenshot. Log:" >&2
  cat "$APP_LOG" >&2 || true
  exit 1
fi

echo "==> Capturing AppKit screenshot: $SCREENSHOT"
screencapture -x "$SCREENSHOT"

if [[ "$AUTO_SMOKE" == "1" ]]; then
  echo "==> Checking AppKit auto smoke route logs"
  for expected in \
    "ZSClip AppKit auto smoke started" \
    "ZSClip AppKit clipboard text smoke write=true read=true" \
    "ZSClip AppKit clipboard file smoke write=true read=true" \
    "ZSClip AppKit clipboard sequence smoke" \
    "changed=true" \
    "ZSClip AppKit clipboard monitor smoke changed=true" \
    "ZSClip AppKit shell open smoke dry_run=$SHELL_OPEN_DRY_RUN_LOG recorded=true" \
    "ZSClip AppKit file picker smoke injected=true recorded=true selected=true" \
    "ZSClip AppKit identity smoke queried=true" \
    "ZSClip AppKit action open_settings -> zsclip.window.open_settings" \
    "ZSClip AppKit auto smoke dialog action dialog_show_info_message -> zsclip.dialog.show_info_message accepted=true" \
    "ZSClip AppKit auto smoke dialog action dialog_confirm_question -> zsclip.dialog.confirm_cancel accepted=true" \
    "ZSClip AppKit auto smoke edit save item_id=" \
    "accepted=true read_back=true" \
    "ZSClip AppKit row action row_copy" \
    "ZSClip AppKit row action row_edit" \
    "ZSClip AppKit edit window shown" \
    "ZSClip AppKit edit save item_id=" \
    "ZSClip AppKit row action row_text_translate" \
    "ZSClip AppKit settings control action settings_toggle_clipboard_capture -> zsclip.settings.toggle_control" \
    "ZSClip AppKit settings control action settings_toggle_lan_sync -> zsclip.settings.toggle_control" \
    "ZSClip AppKit VV select 0 -> vv_select_requested" \
    "ZSClip AppKit VV paste 0 -> zsclip.vv_paste.clipboard_target accepted=true" \
    "ZSClip AppKit status menu action status_toggle_lan_sync -> zsclip.tray.toggle_lan_sync" \
    "ZSClip AppKit auto smoke finished"
  do
    if ! grep -Fq "$expected" "$APP_LOG"; then
      echo "Missing expected AppKit auto smoke log: $expected" >&2
      echo "App log:" >&2
      cat "$APP_LOG" >&2 || true
      exit 1
    fi
  done
fi

# Each scene owns a fresh process and data directory. Readiness comes from the
# actual target UI after its selected page/window has been presented.
# Route smoke and rendered UI evidence are intentionally reported separately.
cleanup
for scene in main settings-general settings-appearance settings-clipboard settings-hotkey settings-group settings-plugin settings-cloud settings-about vv edit; do
  scene_log="$ARTIFACT_DIR/scene-${scene}.log"
  scene_image="$ARTIFACT_DIR/scene-${scene}.png"
  scene_profile="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/zsclip-native-scenes-$$/${scene}"
  mkdir -p "$scene_profile"
  ZSCLIP_DATA_DIR="$scene_profile" ZSCLIP_NATIVE_SETTINGS_FILE="$scene_profile/settings.json" ZSCLIP_NATIVE_HOST_AUTO_SMOKE=1 \
    ZSCLIP_NATIVE_HOST_SCREENSHOT_SCENE="$scene" \
    ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN="$SHELL_OPEN_DRY_RUN" \
    "$ROOT_DIR/target/debug/zsclip" >"$scene_log" 2>&1 &
  APP_PID=$!
  ready=0
  for attempt in $(seq 1 100); do
    if grep -Fq "ZSClip AppKit screenshot scene ready=$scene" "$scene_log"; then
      ready=1
      break
    fi
    if ! kill -0 "$APP_PID" >/dev/null 2>&1; then break; fi
    sleep 0.2
  done
  if [[ "$ready" != "1" ]]; then
    echo "Native screenshot scene did not become ready: $scene" >&2
    cat "$scene_log" >&2
    exit 1
  fi
  sleep 0.5
  screencapture -x "$scene_image"
  test -s "$scene_image"
  cleanup
done

echo "OK: macOS AppKit native host smoke artifacts in $ARTIFACT_DIR"
