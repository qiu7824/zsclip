"""Independent X11 editor fixture for end-to-end native VV paste tests."""

import argparse
import json
import os
from pathlib import Path
import sqlite3
import time

TITLE = "ZSClip Native Paste Receiver"
PREFIX = "DRAFT BEFORE\n"
SUFFIX = "\nDRAFT AFTER"
RUN_ID = os.environ.get("ZSCLIP_NATIVE_RECEIVER_RUN_ID", "standalone")
PAYLOAD = f"ZSCLIP_NATIVE_VV_PAYLOAD_{RUN_ID}\nSecond payload line"


def write_json(path, value):
    path = Path(path)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(temporary, path)


def serve(artifact_dir, timeout):
    import tkinter as tk

    artifact_dir = Path(artifact_dir)
    artifact_dir.mkdir(parents=True, exist_ok=True)
    root = tk.Tk()
    root.title(TITLE)
    root.geometry("720x380+40+40")
    tk.Label(root, text="Native paste recipient", font=("sans", 16, "bold")).pack(pady=12)
    editor = tk.Text(root, wrap="word", font=("monospace", 14), undo=True)
    editor.pack(fill="both", expand=True, padx=20, pady=12)
    editor.insert("1.0", PREFIX + SUFFIX)
    editor.mark_set("insert", "2.0")
    editor.focus_set()
    paste_events = 0
    key_events = []
    case_id = "paste"
    started = time.monotonic()

    def snapshot():
        nonlocal paste_events, case_id
        try:
            request = json.loads((artifact_dir / "receiver-control.json").read_text(encoding="utf-8"))
            if request.get("case_id") and request["case_id"] != case_id:
                case_id = request["case_id"]
                editor.delete("1.0", "end")
                editor.insert("1.0", PREFIX + SUFFIX)
                editor.mark_set("insert", "2.0")
                paste_events = 0
                key_events.clear()
        except (OSError, json.JSONDecodeError):
            pass
        value = {
            "pid": os.getpid(),
            "window_title": TITLE,
            "text": editor.get("1.0", "end-1c"),
            "paste_events": paste_events,
            "key_events": key_events[-64:],
            "case_id": case_id,
            "cursor": editor.index("insert"),
            "elapsed_seconds": round(time.monotonic() - started, 3),
        }
        write_json(artifact_dir / "receiver-result.json", value)
        root.after(100, snapshot)

    def pasted(_event):
        nonlocal paste_events
        paste_events += 1

    def key_pressed(event):
        key_events.append({"key": event.keysym, "state": event.state, "phase": "down"})

    def key_released(event):
        key_events.append({"key": event.keysym, "state": event.state, "phase": "up"})

    def control_digit(event):
        key_pressed(event)
        return "break"

    editor.bind("<<Paste>>", pasted, add="+")
    editor.bind("<KeyPress>", key_pressed, add="+")
    editor.bind("<KeyRelease>", key_released, add="+")
    editor.bind("<Control-Key-1>", control_digit)
    root.update()
    root.clipboard_clear()
    root.clipboard_append(PAYLOAD)
    root.lift()
    editor.focus_force()
    root.update()
    write_json(artifact_dir / "receiver-ready.json", {
        "pid": os.getpid(), "window_title": TITLE, "widget_id": editor.winfo_id(),
        "payload": PAYLOAD, "prefix": PREFIX, "suffix": SUFFIX,
    })
    snapshot()
    root.after(int(timeout * 1000), root.destroy)
    root.mainloop()


def assert_captured(database, output):
    deadline = time.monotonic() + 15
    last_error = None
    while time.monotonic() < deadline:
        try:
            uri = Path(database).resolve().as_uri() + "?mode=ro"
            with sqlite3.connect(uri, uri=True, timeout=0.2) as connection:
                row = connection.execute(
                    "SELECT id,kind,text_data FROM items WHERE category=0 AND text_data=? ORDER BY id DESC LIMIT 1", (PAYLOAD,)
                ).fetchone()
            if row is not None:
                write_json(output, {"captured": True, "item_id": row[0], "kind": row[1], "text": row[2]})
                return
        except (sqlite3.Error, OSError) as error:
            last_error = str(error)
        time.sleep(0.1)
    write_json(output, {"captured": False, "error": last_error})
    raise SystemExit("The receiver payload was not captured by the running application")


def assert_received(result, output):
    # The observe-only Linux trigger currently preserves its two literal v keys.
    # The receiver proves delivery and preservation of the pre-existing draft;
    # this assertion does not claim that trigger cleanup is implemented.
    expected = PREFIX + "vv" + PAYLOAD + SUFFIX
    deadline = time.monotonic() + 12
    observed = None
    while time.monotonic() < deadline:
        try:
            observed = json.loads(Path(result).read_text(encoding="utf-8"))
            if observed.get("text") == expected and observed.get("paste_events", 0) >= 1:
                write_json(output, {
                    "delivered": True, "draft_preserved": True,
                    "trigger_characters_retained": True,
                    "expected": expected, "actual": observed["text"],
                    "paste_events": observed["paste_events"],
                })
                return
        except (OSError, json.JSONDecodeError):
            pass
        time.sleep(0.1)
    write_json(output, {"delivered": False, "expected": expected, "observed": observed})
    raise SystemExit("The independent receiver did not receive exactly the expected paste with its draft intact")


def reset_case(artifact_dir, case_id):
    artifact_dir = Path(artifact_dir)
    write_json(artifact_dir / "receiver-control.json", {"case_id": case_id})
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        try:
            observed = json.loads((artifact_dir / "receiver-result.json").read_text(encoding="utf-8"))
            if observed.get("case_id") == case_id and observed.get("text") == PREFIX + SUFFIX:
                return
        except (OSError, json.JSONDecodeError):
            pass
        time.sleep(0.1)
    raise SystemExit("Recipient did not reset its synthetic draft before the next case")


def assert_cancelled(result, output, case_id, middle, forbidden_key, require_control_digit):
    expected = PREFIX + middle + SUFFIX
    observed = json.loads(Path(result).read_text(encoding="utf-8"))
    keys = observed.get("key_events", [])
    forbidden_leaked = forbidden_key and any(event.get("key") == forbidden_key for event in keys)
    modifier_delivered = not require_control_digit or any(event.get("key") == "1" and event.get("phase") == "down" and event.get("state", 0) & 4 for event in keys)
    passed = observed.get("case_id") == case_id and observed.get("text") == expected and observed.get("paste_events") == 0 and not forbidden_leaked and modifier_delivered
    write_json(output, {"passed": bool(passed), "case_id": case_id, "expected": expected, "observed": observed,
                       "forbidden_key_leaked": bool(forbidden_leaked), "modified_digit_delivered": modifier_delivered})
    if not passed:
        raise SystemExit("Cancellation or key ownership changed the recipient draft unexpectedly")


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    receiver = commands.add_parser("serve")
    receiver.add_argument("--artifact-dir", required=True)
    receiver.add_argument("--timeout", type=float, default=60)
    captured = commands.add_parser("assert-captured")
    captured.add_argument("--database", required=True)
    captured.add_argument("--output", required=True)
    received = commands.add_parser("assert-received")
    received.add_argument("--result", required=True)
    received.add_argument("--output", required=True)
    reset = commands.add_parser("reset")
    reset.add_argument("--artifact-dir", required=True)
    reset.add_argument("--case-id", required=True)
    cancelled = commands.add_parser("assert-cancelled")
    cancelled.add_argument("--result", required=True)
    cancelled.add_argument("--output", required=True)
    cancelled.add_argument("--case-id", required=True)
    cancelled.add_argument("--middle", default="vv")
    cancelled.add_argument("--forbidden-key", default="")
    cancelled.add_argument("--require-control-digit", action="store_true")
    args = parser.parse_args()
    if args.command == "serve":
        serve(args.artifact_dir, args.timeout)
    elif args.command == "assert-captured":
        assert_captured(args.database, args.output)
    elif args.command == "assert-received":
        assert_received(args.result, args.output)
    elif args.command == "reset":
        reset_case(args.artifact_dir, args.case_id)
    else:
        assert_cancelled(args.result, args.output, args.case_id, args.middle, args.forbidden_key, args.require_control_digit)


if __name__ == "__main__":
    main()
