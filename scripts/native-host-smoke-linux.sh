#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "native-host-smoke-linux.sh must run on Linux" >&2
  exit 2
fi

if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
  echo "A Linux GUI session is required. Set DISPLAY or WAYLAND_DISPLAY before running this smoke test." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${ARTIFACT_DIR:-"$ROOT_DIR/target/native-host-smoke/linux"}"
AUTO_SMOKE="${ZSCLIP_NATIVE_HOST_AUTO_SMOKE:-1}"
SHELL_OPEN_DRY_RUN="${ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN:-1}"
if [[ "$SHELL_OPEN_DRY_RUN" == "1" ]]; then
  SHELL_OPEN_DRY_RUN_LOG=true
else
  SHELL_OPEN_DRY_RUN_LOG=false
fi
APP_LOG="$ARTIFACT_DIR/zsclip-gtk.log"
SCREENSHOT="$ARTIFACT_DIR/zsclip-gtk-main.png"
RECEIVER_PID=""

mkdir -p "$ARTIFACT_DIR"
cd "$ROOT_DIR"

echo "==> Linux GTK native host tests"
cargo test -q linux_native_host_launch_plan_targets_real_gtk_entry
cargo test -q linux_native_host_actions_enter_product_command_routes
cargo test -q linux_native_row_actions_enter_product_command_routes
cargo test -q linux_native_status_menu_actions_enter_product_command_routes
cargo test -q linux_native_settings_control_actions_enter_product_command_routes
cargo test -q linux_native_search_text_enters_product_command_route
cargo test -q linux_native_vv_select_enters_product_event_bridge

echo "==> Linux GTK build"
cargo build -q --bin zsclip

echo "==> Launching ZSClip GTK host"
initial_profile="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/zsclip-native-scenes-$$/routes"
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

capture_screenshot() {
  local output="$1"
  if command -v gnome-screenshot >/dev/null 2>&1; then
    gnome-screenshot -f "$output"
  elif command -v grim >/dev/null 2>&1; then
    grim "$output"
  elif command -v import >/dev/null 2>&1; then
    import -window root "$output"
  elif command -v scrot >/dev/null 2>&1; then
    scrot "$output"
  else
    echo "Install gnome-screenshot, grim, imagemagick import, or scrot to capture Linux smoke screenshots." >&2
    return 1
  fi
}

echo "==> Capturing GTK screenshot: $SCREENSHOT"
capture_screenshot "$SCREENSHOT"

if [[ "$AUTO_SMOKE" == "1" ]]; then
  echo "==> Checking GTK auto smoke route logs"
  for expected in \
    "ZSClip GTK auto smoke started" \
    "ZSClip GTK clipboard text smoke write=true read=true" \
    "ZSClip GTK clipboard file smoke write=true read=true" \
    "ZSClip GTK clipboard sequence smoke" \
    "changed=true" \
    "ZSClip GTK clipboard monitor smoke changed=true" \
    "ZSClip GTK shell open smoke dry_run=$SHELL_OPEN_DRY_RUN_LOG recorded=true" \
    "ZSClip GTK file picker smoke injected=true recorded=true selected=true" \
    "ZSClip GTK identity smoke queried=true" \
    "ZSClip GTK action open_settings -> zsclip.window.open_settings" \
    "ZSClip GTK auto smoke dialog action dialog_show_info_message -> zsclip.dialog.show_info_message accepted=true" \
    "ZSClip GTK auto smoke dialog action dialog_confirm_question -> zsclip.dialog.confirm_cancel accepted=true" \
    "ZSClip GTK auto smoke edit save item_id=" \
    "accepted=true read_back=true" \
    "ZSClip GTK row action row_copy -> zsclip.row.copy" \
    "ZSClip GTK row action row_edit -> zsclip.row.edit" \
    "ZSClip GTK edit window shown" \
    "ZSClip GTK edit save item_id=" \
    "ZSClip GTK row action row_text_translate -> zsclip.row.text_translate" \
    "ZSClip GTK settings control action settings_toggle_clipboard_capture -> zsclip.settings.toggle_control" \
    "ZSClip GTK settings control action settings_toggle_lan_sync -> zsclip.settings.toggle_control" \
    "ZSClip GTK VV trigger requested" \
    "ZSClip GTK VV select 0 -> vv_select_requested" \
    "ZSClip GTK VV paste 0 -> zsclip.vv_paste." \
    "ZSClip GTK status menu action status_toggle_lan_sync -> zsclip.tray.toggle_lan_sync" \
    "ZSClip GTK auto smoke finished"
  do
    if ! grep -Fq "$expected" "$APP_LOG"; then
      echo "Missing expected GTK auto smoke log: $expected" >&2
      echo "GTK app log:" >&2
      cat "$APP_LOG" >&2 || true
      exit 1
    fi
  done
  if ! grep -Eq '^ZSClip GTK VV paste 0 -> zsclip\.vv_paste\.(no_external_target|no_session) accepted=false' "$APP_LOG"; then
    echo "Self-target VV smoke did not explicitly reject an absent external target." >&2
    cat "$APP_LOG" >&2
    exit 1
  fi
  if ! grep -Fq "ZSClip GTK StatusNotifierItem installed" "$APP_LOG" \
    && ! grep -Fq "ZSClip GTK StatusNotifierItem unavailable:" "$APP_LOG"; then
    echo "GTK StatusNotifierItem was neither installed nor reported unavailable." >&2
    echo "GTK app log:" >&2
    cat "$APP_LOG" >&2 || true
    exit 1
  fi
  if command -v wmctrl >/dev/null 2>&1 && command -v xdotool >/dev/null 2>&1; then
    if ! grep -Fq "always_on_top_supported=true cursor_follow_supported=true" "$APP_LOG"; then
      echo "GTK X11 window command backend did not verify always-on-top and cursor-follow." >&2
      echo "GTK app log:" >&2
      cat "$APP_LOG" >&2 || true
      exit 1
    fi
  fi
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
    if grep -Fq "ZSClip GTK screenshot scene ready=$scene" "$scene_log"; then
      ready=1
      break
    fi
    if ! kill -0 "$APP_PID" >/dev/null 2>&1; then break; fi
    sleep 0.2
  done
  if [[ "$ready" != "1" ]]; then
    echo "Native screenshot scene did not become ready: $scene" >&2
    capture_screenshot "$ARTIFACT_DIR/scene-${scene}-failure.png" || true
    cat "$scene_log" >&2
    exit 1
  fi
  sleep 0.5
  capture_screenshot "$scene_image"
  test -s "$scene_image"
  cleanup
done

# This phase uses OS input and an independent editor process. Its assertions read
# the recipient text, not ZSClip route logs or an injected success result.
echo "==> Verifying VV delivery into an independent X11 editor"
vv_artifacts="$(mktemp -d "$ARTIFACT_DIR/vv-receiver.XXXXXX")"
vv_profile="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/zsclip-vv-receiver.XXXXXX")"
export ZSCLIP_NATIVE_RECEIVER_RUN_ID="$(basename "$vv_artifacts")"
echo "VV receiver artifacts: $vv_artifacts"
vv_fail() {
  echo "VV receiver verification failed: $1" >&2
  capture_screenshot "$vv_artifacts/failure.png" || true
  if [[ -f "$vv_artifacts/application.log" ]]; then cat "$vv_artifacts/application.log" >&2; fi
  if [[ -f "$vv_artifacts/receiver-result.json" ]]; then cat "$vv_artifacts/receiver-result.json" >&2; fi
  exit 1
}
[[ -n "${DISPLAY:-}" ]] || vv_fail "an X11 DISPLAY is required; pure Wayland is not covered by this test"
command -v xdotool >/dev/null 2>&1 || vv_fail "xdotool is unavailable"
command -v xprop >/dev/null 2>&1 || vv_fail "xprop is unavailable"
python3 -c 'import tkinter' || vv_fail "python3-tk is unavailable"
cat > "$vv_profile/settings.json" <<'JSON'
{"clipboard_capture_enabled":true,"vv_mode_enabled":true,"rich_text_clipboard_enabled":true}
JSON
ZSCLIP_DATA_DIR="$vv_profile" ZSCLIP_NATIVE_SETTINGS_FILE="$vv_profile/settings.json" \
  ZSCLIP_NATIVE_HOST_AUTO_SMOKE=0 ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=1 \
  "$ROOT_DIR/target/debug/zsclip" >"$vv_artifacts/application.log" 2>&1 &
APP_PID=$!
sleep 2
kill -0 "$APP_PID" >/dev/null 2>&1 || vv_fail "the application exited before the recipient started"
python3 "$ROOT_DIR/tests/native_vv_receiver.py" serve --artifact-dir "$vv_artifacts" >"$vv_artifacts/receiver.log" 2>&1 &
RECEIVER_PID=$!
for attempt in $(seq 1 50); do
  [[ -s "$vv_artifacts/receiver-ready.json" ]] && break
  kill -0 "$RECEIVER_PID" >/dev/null 2>&1 || vv_fail "the recipient exited during startup"
  sleep 0.1
done
[[ -s "$vv_artifacts/receiver-ready.json" ]] || vv_fail "the recipient did not become ready"
python3 "$ROOT_DIR/tests/native_vv_receiver.py" assert-captured \
  --database "$vv_profile/clipboard.db" --output "$vv_artifacts/capture.json" \
  || vv_fail "the real clipboard capture did not store the recipient payload"
receiver_window="$(xdotool search --onlyvisible --name '^ZSClip Native Paste Receiver$' | tail -n 1 || true)"
[[ -n "$receiver_window" ]] || vv_fail "the recipient X11 window was not found"
# Tk does not set _NET_WM_PID on every window-manager combination. This is the
# test application's own PID metadata, used by ZSClip to bind its paste target.
xprop -id "$receiver_window" -f _NET_WM_PID 32c -set _NET_WM_PID "$RECEIVER_PID" >/dev/null \
  || vv_fail "the recipient could not publish its own PID metadata"
xdotool windowactivate --sync "$receiver_window" || vv_fail "the recipient could not be activated"
xdotool windowfocus --sync "$receiver_window" || vv_fail "the recipient could not receive focus"
capture_screenshot "$vv_artifacts/before.png"
xdotool type --clearmodifiers --delay 80 'vv' || vv_fail "the VV trigger could not be typed"
popup_window=""
for attempt in $(seq 1 50); do
  popup_window="$(xdotool search --onlyvisible --name '^ZSClip VV Popup$' | tail -n 1 || true)"
  [[ -n "$popup_window" ]] && break
  sleep 0.1
done
[[ -n "$popup_window" ]] || vv_fail "real global VV input did not open a popup; inspect keytap or desktop permissions"
capture_screenshot "$vv_artifacts/popup.png"
xdotool key --clearmodifiers 1 || vv_fail "the candidate key could not be sent"
python3 "$ROOT_DIR/tests/native_vv_receiver.py" assert-received \
  --result "$vv_artifacts/receiver-result.json" --output "$vv_artifacts/verification.json" \
  || vv_fail "recipient contents differ from the expected payload and preserved draft"
capture_screenshot "$vv_artifacts/received.png"
test -s "$vv_artifacts/received.png"

open_receiver_vv_case() {
  local case_id="$1"
  python3 "$ROOT_DIR/tests/native_vv_receiver.py" reset --artifact-dir "$vv_artifacts" --case-id "$case_id" \
    || vv_fail "the recipient could not reset for $case_id"
  xdotool windowactivate --sync "$receiver_window" || vv_fail "the recipient could not be reactivated"
  xdotool windowfocus --sync "$receiver_window" || vv_fail "the recipient focus could not be restored"
  xdotool type --clearmodifiers --delay 80 'vv' || vv_fail "the next VV trigger could not be typed"
  local popup=""
  for attempt in $(seq 1 50); do
    popup="$(xdotool search --onlyvisible --name '^ZSClip VV Popup$' | tail -n 1 || true)"
    [[ -n "$popup" ]] && break
    sleep 0.1
  done
  [[ -n "$popup" ]] || vv_fail "the popup did not open for $case_id"
}
assert_receiver_popup_closed() {
  local remaining
  remaining="$(xdotool search --onlyvisible --name '^ZSClip VV Popup$' || true)"
  [[ -z "$remaining" ]] || vv_fail "a cancelled VV popup remained visible"
}
open_receiver_vv_case held-escape
xdotool keydown Escape || vv_fail "Escape could not be held"
sleep 0.8
xdotool keyup Escape || vv_fail "Escape could not be released"
sleep 0.3
assert_receiver_popup_closed
python3 "$ROOT_DIR/tests/native_vv_receiver.py" assert-cancelled --result "$vv_artifacts/receiver-result.json" \
  --output "$vv_artifacts/held-escape.json" --case-id held-escape --forbidden-key Escape \
  || vv_fail "Escape down/repeat/up leaked to the recipient"
capture_screenshot "$vv_artifacts/held-escape.png"
open_receiver_vv_case ordinary-letter
xdotool type --clearmodifiers 'a' || vv_fail "ordinary input could not be typed"
sleep 0.3
assert_receiver_popup_closed
python3 "$ROOT_DIR/tests/native_vv_receiver.py" assert-cancelled --result "$vv_artifacts/receiver-result.json" \
  --output "$vv_artifacts/ordinary-letter.json" --case-id ordinary-letter --middle vva \
  || vv_fail "ordinary input was swallowed or duplicated"
capture_screenshot "$vv_artifacts/ordinary-letter.png"
open_receiver_vv_case modified-digit
xdotool key --clearmodifiers ctrl+1 || vv_fail "modified digit could not be sent"
sleep 0.3
assert_receiver_popup_closed
python3 "$ROOT_DIR/tests/native_vv_receiver.py" assert-cancelled --result "$vv_artifacts/receiver-result.json" \
  --output "$vv_artifacts/modified-digit.json" --case-id modified-digit --require-control-digit \
  || vv_fail "Ctrl+1 was consumed as an unmodified selection"
capture_screenshot "$vv_artifacts/modified-digit.png"
cleanup
RECEIVER_PID=""

echo "OK: Linux GTK native host smoke artifacts in $ARTIFACT_DIR"
