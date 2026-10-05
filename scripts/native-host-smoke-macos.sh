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
initial_profile="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/zsclip-native-scenes-$$/baseline"
mkdir -p "$initial_profile"
ZSCLIP_DATA_DIR="$initial_profile" ZSCLIP_NATIVE_SETTINGS_FILE="$initial_profile/settings.json" ZSCLIP_NATIVE_HOST_AUTO_SMOKE="$AUTO_SMOKE" ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN="$SHELL_OPEN_DRY_RUN" "$ROOT_DIR/target/debug/zsclip" >"$APP_LOG" 2>&1 &
APP_PID=$!

cleanup() {
  if [[ -n "${RECEIVER_PID:-}" ]] && kill -0 "$RECEIVER_PID" >/dev/null 2>&1; then
    kill "$RECEIVER_PID" >/dev/null 2>&1 || true
    wait "$RECEIVER_PID" >/dev/null 2>&1 || true
  fi
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
    "ZSClip AppKit auto smoke VV self-target rejected=true" \
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

# Clipboard publication and posting Command-V are not evidence of delivery.
# This separate AppKit process reports only the text actually received by its editor.
echo "==> Verifying VV delivery into an independent AppKit editor"
vv_artifacts="$(mktemp -d "$ARTIFACT_DIR/vv-receiver.XXXXXX")"
vv_profile="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/zsclip-mac-vv-profile.XXXXXX")"
receiver_binary="$vv_profile/native-vv-receiver"
vv_payload="VV-DELIVERY-$(date +%s)-$$"
xcrun swiftc "$ROOT_DIR/scripts/native-vv-receiver-macos.swift" -o "$receiver_binary" \
  >"$vv_artifacts/receiver-build.log" 2>&1
ZSCLIP_VV_RECEIVER_PAYLOAD="$vv_payload" ZSCLIP_VV_PUBLISH_AFTER=4 \
  "$receiver_binary" "$vv_artifacts/receiver-state.json" >"$vv_artifacts/receiver.log" 2>&1 &
RECEIVER_PID=$!
for attempt in $(seq 1 50); do
  [[ -s "$vv_artifacts/receiver-state.json" ]] && break
  kill -0 "$RECEIVER_PID" >/dev/null 2>&1 || break
  sleep 0.1
done
test -s "$vv_artifacts/receiver-state.json"
receiver_pid="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["pid"])' "$vv_artifacts/receiver-state.json")"
cat > "$vv_profile/settings.json" <<'JSON'
{"clipboard_capture_enabled":true,"vv_mode_enabled":true,"vv_source_tab":0,"vv_group_id":0,"lan_sync_enabled":false,"cloud_sync_enabled":false,"auto_start":false}
JSON
ZSCLIP_DATA_DIR="$vv_profile" ZSCLIP_NATIVE_SETTINGS_FILE="$vv_profile/settings.json" \
  ZSCLIP_NATIVE_HOST_AUTO_SMOKE=0 ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=1 \
  ZSCLIP_NATIVE_VV_RECEIVER_PID="$receiver_pid" ZSCLIP_NATIVE_VV_DELIVERY_SMOKE=1 \
  ZSCLIP_VV_RECEIVER_PAYLOAD="$vv_payload" \
  "$ROOT_DIR/target/debug/zsclip" >"$vv_artifacts/application.log" 2>&1 &
APP_PID=$!
set +e
python3 - "$vv_artifacts" "$vv_payload" <<'PY'
import json,sys,time
from pathlib import Path
folder=Path(sys.argv[1])
expected="LEFT-"+sys.argv[2]+"RIGHT"
deadline=time.monotonic()+30
state=None
while time.monotonic()<deadline:
    try:
        state=json.loads((folder/"receiver-state.json").read_text())
        if state.get("text")==expected:
            (folder/"verification.json").write_text(json.dumps({"delivered":True,"draft_preserved":True,"state":state},indent=2))
            break
    except (OSError,json.JSONDecodeError):
        pass
    time.sleep(0.1)
else:
    (folder/"verification.json").write_text(json.dumps({"delivered":False,"expected":expected,"state":state},indent=2))
    raise SystemExit("The independent AppKit editor did not receive the selected VV payload")
PY
vv_status=$?
set -e
screencapture -x "$vv_artifacts/received.png"
if [[ "$vv_status" != 0 ]]; then
  cat "$vv_artifacts/application.log" >&2
  cat "$vv_artifacts/verification.json" >&2
  exit "$vv_status"
fi
cleanup
RECEIVER_PID=""

echo "OK: macOS AppKit native host smoke artifacts in $ARTIFACT_DIR"
