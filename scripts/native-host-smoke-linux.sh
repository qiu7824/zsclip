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
    "ZSClip GTK VV paste 0 -> zsclip.vv_paste.clipboard_target accepted=true" \
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
    cat "$scene_log" >&2
    exit 1
  fi
  sleep 0.5
  capture_screenshot "$scene_image"
  test -s "$scene_image"
  cleanup
done

echo "OK: Linux GTK native host smoke artifacts in $ARTIFACT_DIR"
