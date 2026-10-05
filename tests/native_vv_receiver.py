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
    started = time.monotonic()

    def snapshot():
        value = {
            "pid": os.getpid(),
            "window_title": TITLE,
            "text": editor.get("1.0", "end-1c"),
            "paste_events": paste_events,
            "key_events": key_events[-40:],
            "cursor": editor.index("insert"),
            "elapsed_seconds": round(time.monotonic() - started, 3),
        }
        write_json(artifact_dir / "receiver-result.json", value)
        root.after(100, snapshot)

    def pasted(_event):
        nonlocal paste_events
        paste_events += 1

    def key_pressed(event):
        key_events.append({"key": event.keysym, "state": event.state})

    editor.bind("<<Paste>>", pasted, add="+")
    editor.bind("<KeyPress>", key_pressed, add="+")
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
    args = parser.parse_args()
    if args.command == "serve":
        serve(args.artifact_dir, args.timeout)
    elif args.command == "assert-captured":
        assert_captured(args.database, args.output)
    else:
        assert_received(args.result, args.output)


if __name__ == "__main__":
    main()
