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
    "ZSClip AppKit auto smoke native rows copy_verified=true edit_verified=true" \
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

echo "==> Verifying global shortcuts and selected-history delivery"
for hotkey_mode in normal plain; do
  hotkey_artifacts="$(mktemp -d "$ARTIFACT_DIR/hotkey-${hotkey_mode}.XXXXXX")"
  hotkey_profile="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/zsclip-mac-hotkey-${hotkey_mode}.XXXXXX")"
  mkdir -p "$hotkey_artifacts"
  hotkey_payload="HOTKEY-${hotkey_mode}-$(date +%s)-$$"
  cat > "$hotkey_profile/settings.json" <<'JSON'
{"clipboard_capture_enabled":true,"rich_text_clipboard_enabled":true,"hotkey_enabled":true,"hotkey_mod":"Ctrl+Alt","hotkey_key":"V","plain_paste_hotkey_enabled":true,"plain_paste_hotkey_mod":"Ctrl+Shift","plain_paste_hotkey_key":"V","vv_mode_enabled":false,"lan_sync_enabled":false,"cloud_sync_enabled":false,"auto_start":false}
JSON
  ZSCLIP_DATA_DIR="$hotkey_profile" ZSCLIP_NATIVE_SETTINGS_FILE="$hotkey_profile/settings.json" \
    ZSCLIP_NATIVE_HOST_AUTO_SMOKE=0 ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=1 ZSCLIP_NATIVE_HOTKEY_SMOKE=1 \
    "$ROOT_DIR/target/debug/zsclip" >"$hotkey_artifacts/application.log" 2>&1 &
  APP_PID=$!
  # Finish the host's startup activation before opening the external editor.
  # Otherwise a late didFinishLaunching activation steals focus back from it.
  host_ready=0
  for attempt in $(seq 1 100); do
    if grep -Fq 'ZSClip AppKit main list ready mode=normal' "$hotkey_artifacts/application.log"; then
      host_ready=1
      break
    fi
    if ! kill -0 "$APP_PID" >/dev/null 2>&1; then break; fi
    sleep 0.1
  done
  if [[ "$host_ready" != "1" ]]; then
    cat "$hotkey_artifacts/application.log" >&2
    exit 1
  fi
  if ! "$receiver_binary" --wait-frontmost "$APP_PID" >"$hotkey_artifacts/startup-activation.log" 2>&1; then
    cat "$hotkey_artifacts/startup-activation.log" >&2
    cat "$hotkey_artifacts/application.log" >&2
    exit 1
  fi
  ZSCLIP_VV_RECEIVER_PAYLOAD="$hotkey_payload" ZSCLIP_VV_PUBLISH_AFTER=1 ZSCLIP_RECEIVER_PUBLISH_HTML=1 \
    "$receiver_binary" "$hotkey_artifacts/receiver-state.json" >"$hotkey_artifacts/receiver.log" 2>&1 &
  RECEIVER_PID=$!
  set +e
  python3 - "$hotkey_artifacts" "$hotkey_payload" "$hotkey_mode" "$receiver_binary" <<'PY'
import json, subprocess, sys, time
from pathlib import Path
folder, payload, mode, helper = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4]
state_path = folder / 'receiver-state.json'
log_path = folder / 'application.log'
def wait_for(predicate, description):
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        try:
            state = json.loads(state_path.read_text())
            log = log_path.read_text()
            if predicate(state, log): return state, log
        except (OSError, json.JSONDecodeError): pass
        time.sleep(0.1)
    raise RuntimeError(description)
try:
    state, log = wait_for(lambda s,l: s.get('clipboard_published') and
                          ('clipboard capture sequence=' + str(s.get('published_sequence')) + ' inserted=true') in l,
                          'The externally published rich-text record was not captured')
    original_text = state.get('text')
    original_selection = (state.get('selection_location'), state.get('selection_length'))
    if original_text != 'LEFT-RIGHT' or original_selection != (5, 0):
        raise RuntimeError('The receiver did not start with the expected synthetic draft and caret')
    activated, log = wait_for(lambda s,l: s.get('active') and s.get('key_window') and s.get('first_responder_is_editor')
                             and s.get('frontmost_pid') == s.get('pid'),
                             'The receiver did not become the active editor')
    if activated.get('text') != original_text or (
            activated.get('selection_location'), activated.get('selection_length')) != original_selection:
        raise RuntimeError('Receiver activation changed the draft or insertion point')
    subprocess.run([helper, '--send', mode], check=True)
    opened = 'ZSClip AppKit global shortcut opened mode=' + mode
    ready = 'ZSClip AppKit main list ready mode=' + mode
    state, log = wait_for(lambda s,l: opened in l and ready in l.split(opened,1)[1],
                          'The actual global shortcut did not open a ready history list')
    if state['text'] != 'LEFT-RIGHT': raise RuntimeError('Shortcut input leaked into the draft')
    if log.count(opened) != 1: raise RuntimeError('A repeated keydown reopened the list')
    subprocess.run([helper, '--send', 'return'], check=True)
    expected = 'LEFT-' + payload + 'RIGHT'
    state, log = wait_for(lambda s,l: s.get('text') == expected and
                          any(e.get('code') == 9 and e.get('phase') == 'up' for e in s.get('key_events', [])),
                          'The selected history record did not reach the external editor')
    time.sleep(0.3)
    state = json.loads(state_path.read_text())
    log = log_path.read_text()
    if state.get('text') != expected: raise RuntimeError('Late input changed the received draft')
    v_events = [e for e in state.get('key_events', []) if e.get('code') == 9]
    shortcut_mask = (1 << 17) | (1 << 18) | (1 << 19) | (1 << 20)
    if [e.get('phase') for e in v_events] != ['down', 'up'] or any(
            (e.get('modifiers', 0) & shortcut_mask) != (1 << 20) or e.get('repeat') for e in v_events):
        raise RuntimeError('The receiver saw a leaked trigger cycle or an invalid Command-V cycle')
    if log.count(opened) != 1: raise RuntimeError('The shortcut retriggered after key release')
    if state.get('paste_count') != 1: raise RuntimeError('Selection did not produce exactly one native paste')
    if state.get('clipboard_has_html') != (mode == 'normal'):
        raise RuntimeError('The selected paste mode did not preserve/remove HTML as required')
    result = {'delivered': True, 'draft_preserved': True, 'mode': mode,
              'activation_preserved_selection': True, 'shortcut_repeats_consumed': True,
              'format_verified': True, 'state': state}
    (folder/'verification.json').write_text(json.dumps(result, indent=2))
except Exception as error:
    try: state = json.loads(state_path.read_text())
    except Exception: state = None
    (folder/'verification.json').write_text(json.dumps({'delivered': False, 'error': str(error), 'state': state}, indent=2))
    raise
PY
  hotkey_status=$?
  set -e
  screencapture -x "$hotkey_artifacts/received.png"
  if [[ "$hotkey_status" != 0 ]]; then
    cat "$hotkey_artifacts/application.log" >&2
    cat "$hotkey_artifacts/verification.json" >&2
    exit "$hotkey_status"
  fi
  cleanup
  RECEIVER_PID=""
done

echo "OK: macOS AppKit native host smoke artifacts in $ARTIFACT_DIR"
