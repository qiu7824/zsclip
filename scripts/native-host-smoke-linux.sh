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
CLICK_SMOKE="${NATIVE_HOST_SMOKE_CLICK:-0}"
SHELL_OPEN_DRY_RUN="${ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN:-1}"
if [[ "$SHELL_OPEN_DRY_RUN" == "1" ]]; then
  SHELL_OPEN_DRY_RUN_LOG=true
else
  SHELL_OPEN_DRY_RUN_LOG=false
fi
APP_LOG="$ARTIFACT_DIR/zsclip-gtk.log"
SCREENSHOT="$ARTIFACT_DIR/zsclip-gtk-main.png"
CLICK_SCREENSHOT="$ARTIFACT_DIR/zsclip-gtk-after-clicks.png"

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
    "ZSClip GTK VV native paste shortcut posted=" \
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
fi

if [[ "$CLICK_SMOKE" == "1" ]]; then
  if ! command -v xdotool >/dev/null 2>&1; then
    echo "Install xdotool to run optional GTK click smoke." >&2
    exit 1
  fi
  echo "==> Running optional GTK button click smoke"
  WINDOW_ID="$(xdotool search --name '^ZSClip$' | head -n 1 || true)"
  if [[ -z "$WINDOW_ID" ]]; then
    echo "Could not find a ZSClip window for click smoke." >&2
    exit 1
  fi
  xdotool windowactivate "$WINDOW_ID"
  sleep 0.2
  xdotool mousemove --window "$WINDOW_ID" 70 90 click 1
  sleep 0.2
  xdotool key Escape
  sleep 0.2
  xdotool mousemove --window "$WINDOW_ID" 150 260 click 1
  sleep 0.2
  xdotool mousemove --window "$WINDOW_ID" 260 260 click 1
  sleep 0.2
  SETTINGS_WINDOW_ID="$(xdotool search --name '^ZSClip Settings$' | head -n 1 || true)"
  if [[ -n "$SETTINGS_WINDOW_ID" ]]; then
    xdotool windowactivate "$SETTINGS_WINDOW_ID"
    sleep 0.2
    xdotool mousemove --window "$SETTINGS_WINDOW_ID" 90 420 click 1
    sleep 0.2
    xdotool mousemove --window "$SETTINGS_WINDOW_ID" 230 420 click 1
    sleep 0.2
    xdotool mousemove --window "$SETTINGS_WINDOW_ID" 510 420 click 1
    sleep 0.2
    xdotool windowactivate "$WINDOW_ID"
    sleep 0.2
  fi
  xdotool mousemove --window "$WINDOW_ID" 150 330 click 1
  sleep 0.2
  xdotool mousemove --window "$WINDOW_ID" 600 330 click 1
  sleep 0.5
  VV_WINDOW_ID="$(xdotool search --name '^ZSClip VV Popup$' | head -n 1 || true)"
  if [[ -n "$VV_WINDOW_ID" ]]; then
    xdotool windowactivate "$VV_WINDOW_ID"
    sleep 0.2
    xdotool mousemove --window "$VV_WINDOW_ID" 320 136 click 1
    sleep 0.2
    xdotool windowactivate "$WINDOW_ID"
    sleep 0.2
  fi
  for expected in \
    "ZSClip GTK row action row_copy -> zsclip.row.copy" \
    "ZSClip GTK row action row_text_translate -> zsclip.row.text_translate" \
    "ZSClip GTK settings control action settings_toggle_clipboard_capture -> zsclip.settings.toggle_control" \
    "ZSClip GTK settings control action settings_toggle_lan_sync -> zsclip.settings.toggle_control"
  do
    if ! grep -Fq "$expected" "$APP_LOG"; then
      echo "Missing expected GTK route log: $expected" >&2
      echo "GTK app log:" >&2
      cat "$APP_LOG" >&2 || true
      exit 1
    fi
  done
  if grep -Fq "ZSClip GTK VV trigger requested" "$APP_LOG" \
    && ! grep -Fq "ZSClip GTK VV select 0 -> vv_select_requested" "$APP_LOG"; then
    echo "GTK VV popup opened, but Select 1 did not bridge vv_select_requested." >&2
    cat "$APP_LOG" >&2 || true
    exit 1
  fi
  if grep -Fq "ZSClip GTK VV trigger requested" "$APP_LOG" \
    && ! grep -Fq "ZSClip GTK VV paste 0 -> zsclip.vv_paste.clipboard_target accepted=true" "$APP_LOG"; then
    echo "GTK VV popup opened, but Select 1 did not execute the native VV paste bridge." >&2
    cat "$APP_LOG" >&2 || true
    exit 1
  fi
  if grep -Fq "ZSClip GTK VV paste 0 -> zsclip.vv_paste.clipboard_target accepted=true" "$APP_LOG" \
    && ! grep -Fq "ZSClip GTK VV native paste shortcut posted=" "$APP_LOG"; then
    echo "GTK VV paste bridge ran, but the native paste shortcut path was not attempted." >&2
    cat "$APP_LOG" >&2 || true
    exit 1
  fi
  capture_screenshot "$CLICK_SCREENSHOT"
  echo "OK: GTK click screenshot: $CLICK_SCREENSHOT"
else
  echo "Skipped GTK button clicks. Set NATIVE_HOST_SMOKE_CLICK=1 and install xdotool to run them."
fi

echo "OK: Linux GTK native host smoke artifacts in $ARTIFACT_DIR"
