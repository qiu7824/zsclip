#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "native-host-smoke-macos.sh must run on macOS" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${ARTIFACT_DIR:-"$ROOT_DIR/target/native-host-smoke/macos"}"
AUTO_SMOKE="${ZSCLIP_NATIVE_HOST_AUTO_SMOKE:-1}"
CLICK_SMOKE="${NATIVE_HOST_SMOKE_CLICK:-0}"
SHELL_OPEN_DRY_RUN="${ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN:-1}"
if [[ "$SHELL_OPEN_DRY_RUN" == "1" ]]; then
  SHELL_OPEN_DRY_RUN_LOG=true
else
  SHELL_OPEN_DRY_RUN_LOG=false
fi
APP_LOG="$ARTIFACT_DIR/zsclip-appkit.log"
SCREENSHOT="$ARTIFACT_DIR/zsclip-appkit-main.png"
CLICK_SCREENSHOT="$ARTIFACT_DIR/zsclip-appkit-after-clicks.png"

mkdir -p "$ARTIFACT_DIR"
cd "$ROOT_DIR"

echo "==> macOS AppKit native host tests"
cargo test macos_native_host_launch_plan_targets_real_appkit_entry
cargo test macos_native_host_actions_enter_product_command_routes
cargo test macos_native_row_actions_enter_product_command_routes
cargo test macos_native_status_menu_actions_enter_product_command_routes
cargo test macos_native_settings_control_actions_enter_product_command_routes
cargo test macos_native_search_text_enters_product_command_route
cargo test macos_native_vv_select_enters_product_event_bridge

echo "==> macOS AppKit build"
cargo build --bin zsclip

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
    "ZSClip AppKit row action row_copy -> zsclip.row.copy" \
    "ZSClip AppKit row action row_edit -> zsclip.row.edit" \
    "ZSClip AppKit edit window shown" \
    "ZSClip AppKit edit save item_id=" \
    "ZSClip AppKit row action row_text_translate -> zsclip.row.text_translate" \
    "ZSClip AppKit settings control action settings_toggle_clipboard_capture -> zsclip.settings.toggle_control" \
    "ZSClip AppKit settings control action settings_toggle_lan_sync -> zsclip.settings.toggle_control" \
    "ZSClip AppKit VV select 0 -> vv_select_requested" \
    "ZSClip AppKit VV paste 0 -> zsclip.vv_paste.clipboard_target accepted=true" \
    "ZSClip AppKit VV native paste shortcut posted=" \
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

if [[ "$CLICK_SMOKE" == "1" ]]; then
  echo "==> Running optional AppKit button click smoke"
  if ! osascript <<'APPLESCRIPT'
tell application "System Events"
  tell process "zsclip"
    set frontmost to true
    delay 0.5
    click button "Search" of window "ZSClip"
    delay 0.2
    click button "Settings" of window "ZSClip"
    delay 0.5
    click button "Capture" of window "ZSClip Settings"
    delay 0.2
    click button "LAN Sync" of window "ZSClip Settings"
    delay 0.2
    click button "Sync Mode" of window "ZSClip Settings"
    delay 0.2
    click button "Copy" of window "ZSClip"
    delay 0.2
    click button "Translate" of window "ZSClip"
    delay 0.2
    click button "Row Menu" of window "ZSClip"
    delay 0.2
    key code 53
    delay 0.2
    click button "Group Filter" of window "ZSClip"
    delay 0.2
    key code 53
    delay 0.2
    click button "VV Popup" of window "ZSClip"
    delay 0.5
    click button "Select 1" of window "ZSClip VV Popup"
    delay 0.2
    click menu bar item "ZSClip" of menu bar 2
    delay 0.2
    click menu item "Toggle LAN Sync" of menu 1 of menu bar item "ZSClip" of menu bar 2
  end tell
end tell
APPLESCRIPT
  then
    echo "Optional click smoke failed. Grant Terminal accessibility permission and rerun with NATIVE_HOST_SMOKE_CLICK=1." >&2
    exit 1
  fi
  for expected in \
    "ZSClip AppKit row action row_copy -> zsclip.row.copy" \
    "ZSClip AppKit row action row_text_translate -> zsclip.row.text_translate" \
    "ZSClip AppKit row popup menu shown:" \
    "ZSClip AppKit group filter popup menu shown:" \
    "ZSClip AppKit VV select 0 -> vv_select_requested" \
    "ZSClip AppKit VV paste 0 -> zsclip.vv_paste.clipboard_target accepted=true" \
    "ZSClip AppKit VV native paste shortcut posted=" \
    "ZSClip AppKit status menu action status_toggle_lan_sync -> zsclip.tray.toggle_lan_sync"
  do
    if ! grep -Fq "$expected" "$APP_LOG"; then
      echo "Missing expected AppKit route log: $expected" >&2
      echo "App log:" >&2
      cat "$APP_LOG" >&2 || true
      exit 1
    fi
  done
  screencapture -x "$CLICK_SCREENSHOT"
  echo "OK: AppKit click screenshot: $CLICK_SCREENSHOT"
else
  echo "Skipped AppKit button clicks. Set NATIVE_HOST_SMOKE_CLICK=1 to run them."
fi

echo "OK: macOS AppKit native host smoke artifacts in $ARTIFACT_DIR"
