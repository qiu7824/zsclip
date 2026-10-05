"""Independent X11 editor fixture for end-to-end native VV paste tests."""

import argparse
import html
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
HTML_PAYLOAD = "<p><b>" + html.escape(PAYLOAD).replace("\n", "<br>") + "</b></p>"


def write_json(path, value):
    path = Path(path)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(temporary, path)


class X11ClipboardFixture:
    """Publish and inspect the synthetic clipboard through separate X11 clients."""

    def __init__(self):
        from Xlib import X, Xatom, display, protocol

        self.X, self.Xatom, self.protocol = X, Xatom, protocol
        self.publisher = display.Display()
        self.reader = display.Display()
        self.owner = self.publisher.screen().root.create_window(
            0, 0, 1, 1, 0, X.CopyFromParent, X.InputOutput, X.CopyFromParent)
        self.requestor = self.reader.screen().root.create_window(
            0, 0, 1, 1, 0, X.CopyFromParent, X.InputOutput, X.CopyFromParent)
        self.selection = self.publisher.intern_atom("CLIPBOARD")
        self.property = self.reader.intern_atom("ZSCLIP_RECEIVER_SELECTION")
        self.targets = {
            self.publisher.intern_atom(name): value.encode("utf-8")
            for name, value in [("UTF8_STRING", PAYLOAD), ("STRING", PAYLOAD),
                                ("text/plain;charset=utf-8", PAYLOAD), ("text/html", HTML_PAYLOAD)]
        }
        self.targets_atom = self.publisher.intern_atom("TARGETS")
        self.requests = []

    def publish(self):
        self.owner.set_selection_owner(self.selection, self.X.CurrentTime)
        self.publisher.sync()
        if self.publisher.get_selection_owner(self.selection).id != self.owner.id:
            raise RuntimeError("The fixture could not own the X11 clipboard")

    def poll(self):
        while self.publisher.pending_events():
            event = self.publisher.next_event()
            if event.type != self.X.SelectionRequest:
                continue
            property_atom = event.property or event.target
            payload = self.targets.get(event.target)
            if event.selection != self.selection:
                property_atom = self.X.NONE
            elif event.target == self.targets_atom:
                event.requestor.change_property(property_atom, self.Xatom.ATOM, 32,
                                                [self.targets_atom, *self.targets])
            elif payload is not None:
                # MIME target and returned property type must agree. Tk's
                # clipboard_append(type="text/html") instead returns STRING.
                event.requestor.change_property(property_atom, event.target, 8, payload)
            else:
                property_atom = self.X.NONE
            self.requests.append({"target": self.publisher.get_atom_name(event.target),
                                  "accepted": property_atom != self.X.NONE})
            self.requests = self.requests[-64:]
            event.requestor.send_event(self.protocol.event.SelectionNotify(
                time=event.time, requestor=event.requestor, selection=event.selection,
                target=event.target, property=property_atom), propagate=False)
            self.publisher.flush()

    def read(self, target_name):
        target = self.reader.intern_atom(target_name)
        self.requestor.delete_property(self.property)
        self.requestor.convert_selection(self.selection, target, self.property, self.X.CurrentTime)
        self.reader.flush()
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            self.poll()
            while self.reader.pending_events():
                event = self.reader.next_event()
                if event.type != self.X.SelectionNotify or event.target != target:
                    continue
                if event.property == self.X.NONE:
                    return {"target": target_name, "available": False}
                value = self.requestor.get_full_property(self.property, self.X.AnyPropertyType)
                if value is None:
                    return {"target": target_name, "available": False}
                result = {"target": target_name, "available": True,
                          "returned_type": self.reader.get_atom_name(value.property_type), "format": value.format}
                if value.format == 8:
                    raw = bytes(value.value)
                    result.update(byte_length=len(raw), text=raw.decode("utf-8"))
                elif value.property_type == self.Xatom.ATOM and value.format == 32:
                    result["targets"] = [self.reader.get_atom_name(atom) for atom in value.value]
                return result
            time.sleep(0.005)
        raise RuntimeError(f"Timed out reading X11 clipboard target {target_name}")

    def close(self):
        self.owner.destroy()
        self.requestor.destroy()
        self.publisher.close()
        self.reader.close()


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
    pasted_html = []
    pasted_transfers = []
    case_id = "paste"
    started = time.monotonic()
    clipboard = X11ClipboardFixture()

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
                pasted_html.clear()
                pasted_transfers.clear()
        except (OSError, json.JSONDecodeError):
            pass
        value = {
            "pid": os.getpid(),
            "window_title": TITLE,
            "text": editor.get("1.0", "end-1c"),
            "paste_events": paste_events,
            "key_events": key_events[-64:],
            "pasted_html": pasted_html,
            "pasted_transfers": pasted_transfers,
            "case_id": case_id,
            "cursor": editor.index("insert"),
            "elapsed_seconds": round(time.monotonic() - started, 3),
            "clipboard_requests": clipboard.requests,
        }
        write_json(artifact_dir / "receiver-result.json", value)
        root.after(100, snapshot)

    def pasted(_event):
        nonlocal paste_events
        paste_events += 1
        try:
            value = clipboard.read("text/html")
            pasted_transfers.append(value)
            pasted_html.append(value.get("text") if value.get("returned_type") == "text/html" and value.get("format") == 8 else None)
        except (RuntimeError, UnicodeDecodeError) as error:
            pasted_transfers.append({"error": str(error)})
            pasted_html.append(None)

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
    clipboard.publish()
    offers = {target: clipboard.read(target) for target in ("TARGETS", "UTF8_STRING", "text/html")}
    write_json(artifact_dir / "clipboard-offer.json", offers)
    if not {"UTF8_STRING", "text/html"}.issubset(offers["TARGETS"].get("targets", [])):
        clipboard.close()
        raise SystemExit("The fixture did not advertise its text and HTML selection targets")
    for target, expected in (("UTF8_STRING", PAYLOAD), ("text/html", HTML_PAYLOAD)):
        if offers[target].get("returned_type") != target or offers[target].get("format") != 8 or offers[target].get("text") != expected:
            clipboard.close()
            raise SystemExit(f"The fixture did not publish a valid X11 {target} selection")

    def poll_clipboard():
        clipboard.poll()
        root.after(10, poll_clipboard)

    poll_clipboard()
    root.lift()
    editor.focus_force()
    root.update()
    write_json(artifact_dir / "receiver-ready.json", {
        "pid": os.getpid(), "window_title": TITLE, "widget_id": editor.winfo_id(),
        "payload": PAYLOAD, "prefix": PREFIX, "suffix": SUFFIX,
        "clipboard_targets": offers["TARGETS"].get("targets", []),
        "clipboard_offer_verified": True,
    })
    snapshot()
    root.after(int(timeout * 1000), root.destroy)
    try:
        root.mainloop()
    finally:
        clipboard.close()


def assert_captured(database, output):
    deadline = time.monotonic() + 15
    last_error = None
    diagnostics = {"row_found": False, "text_equals": False, "html_present": False, "html_length": 0}
    while time.monotonic() < deadline:
        try:
            uri = Path(database).resolve().as_uri() + "?mode=ro"
            with sqlite3.connect(uri, uri=True, timeout=0.2) as connection:
                row = connection.execute(
                    "SELECT id,kind,text_data,rich_text_html FROM items WHERE category=0 AND text_data=? ORDER BY id DESC LIMIT 1", (PAYLOAD,)
                ).fetchone()
                latest = connection.execute(
                    "SELECT id,kind,text_data,length(rich_text_html) FROM items WHERE category=0 ORDER BY id DESC LIMIT 4"
                ).fetchall()
            last_error = None
            html_matches = row is not None and bool(row[3]) and all(line in html.unescape(row[3]) for line in PAYLOAD.splitlines())
            diagnostics = {"row_found": row is not None, "text_equals": row is not None and row[2] == PAYLOAD,
                           "html_present": row is not None and bool(row[3]),
                           "html_length": len(row[3] or "") if row is not None else 0,
                           "html_payload_matches": html_matches,
                           "latest_rows": latest, "expected_payload": PAYLOAD}
            if html_matches:
                write_json(output, {"captured": True, "item_id": row[0], "kind": row[1], "text": row[2], "html": row[3], **diagnostics})
                return
        except (sqlite3.Error, OSError) as error:
            last_error = str(error)
        time.sleep(0.1)
    write_json(output, {"captured": False, "error": last_error, **diagnostics})
    raise SystemExit("The receiver payload was not captured by the running application")


def assert_received(result, output, mode="vv", case_id=None):
    # The observe-only Linux trigger currently preserves its two literal v keys.
    # The receiver proves delivery and preservation of the pre-existing draft;
    # this assertion does not claim that trigger cleanup is implemented.
    expected = PREFIX + ("vv" if mode == "vv" else "") + PAYLOAD + SUFFIX
    deadline = time.monotonic() + 12
    observed = None
    while time.monotonic() < deadline:
        try:
            observed = json.loads(Path(result).read_text(encoding="utf-8"))
            html_values = observed.get("pasted_html", [])
            transfers = observed.get("pasted_transfers", [])
            formats_match = mode == "vv" or (html_values == [None] and len(transfers) == 1
                and transfers[0].get("available") is False and "error" not in transfers[0] if mode == "plain" else
                len(html_values) == 1 and isinstance(html_values[0], str)
                and all(line in html.unescape(html_values[0]) for line in PAYLOAD.splitlines()))
            shortcut_leaked = mode != "vv" and any(event.get("key", "").lower() == "v" and not (event.get("state", 0) & 4) for event in observed.get("key_events", []))
            if observed.get("text") == expected and observed.get("paste_events", 0) == 1 and formats_match and not shortcut_leaked and (case_id is None or observed.get("case_id") == case_id):
                write_json(output, {
                    "delivered": True, "draft_preserved": True,
                    "trigger_characters_retained": mode == "vv", "mode": mode,
                    "expected": expected, "actual": observed["text"],
                    "paste_events": observed["paste_events"],
                    "pasted_html": html_values,
                    "pasted_transfers": transfers,
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
    forbidden_leaked = forbidden_key and any(event.get("key", "").casefold() == forbidden_key.casefold() for event in keys)
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
    received.add_argument("--mode", choices=["vv", "normal", "plain"], default="vv")
    received.add_argument("--case-id")
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
        assert_received(args.result, args.output, args.mode, args.case_id)
    elif args.command == "reset":
        reset_case(args.artifact_dir, args.case_id)
    else:
        assert_cancelled(args.result, args.output, args.case_id, args.middle, args.forbidden_key, args.require_control_digit)


if __name__ == "__main__":
    main()
