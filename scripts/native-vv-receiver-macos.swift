import AppKit
import Foundation

// Test driver posts normal system input. It never invokes a ZSClip callback or
// the receiver's paste action; the receiver records what AppKit actually did.
if CommandLine.arguments.count == 3 && CommandLine.arguments[1] == "--activate" {
    // A background command-line helper cannot force activation on recent macOS.
    // Click the receiver's exposed title bar, without touching its draft/caret.
    guard CGPreflightPostEventAccess(),
          let bytes = try? Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[2])),
          let state = (try? JSONSerialization.jsonObject(with: bytes)) as? [String: Any],
          let pid = (state["pid"] as? NSNumber)?.int32Value,
          let windowNumber = (state["window_number"] as? NSNumber)?.intValue,
          let titleBarHeight = (state["titlebar_height"] as? NSNumber)?.doubleValue, titleBarHeight >= 8,
          let app = NSRunningApplication(processIdentifier: pid), !app.isTerminated,
          let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]]
    else { fputs("Receiver activation metadata or event access unavailable\n", stderr); exit(2) }
    func windowBounds(_ info: [String: Any]) -> CGRect? {
        guard let bounds = info[kCGWindowBounds as String] as? [String: NSNumber],
              let bx = bounds["X"]?.doubleValue, let by = bounds["Y"]?.doubleValue,
              let width = bounds["Width"]?.doubleValue, let height = bounds["Height"]?.doubleValue
        else { return nil }
        return CGRect(x: bx, y: by, width: width, height: height)
    }
    guard let receiverWindow = windows.first(where: {
        ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid &&
        ($0[kCGWindowNumber as String] as? NSNumber)?.intValue == windowNumber
    }), let frame = windowBounds(receiverWindow), frame.width > 120 else {
        fputs("Receiver window is absent from the on-screen window list\n", stderr); exit(3)
    }
    func topWindow(at point: CGPoint) -> [String: Any]? {
        windows.first { info in
            ((info[kCGWindowAlpha as String] as? NSNumber)?.doubleValue ?? 1) > 0 &&
            (windowBounds(info)?.contains(point) ?? false)
        }
    }
    var exposedPoint: CGPoint?
    let titleY = frame.minY + min(CGFloat(titleBarHeight / 2), 18)
    for x in stride(from: frame.maxX - 24, through: frame.minX + 90, by: -16) {
        let candidate = CGPoint(x: x, y: titleY)
        let top = topWindow(at: candidate)
        if (top?[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid &&
           (top?[kCGWindowNumber as String] as? NSNumber)?.intValue == windowNumber {
            exposedPoint = candidate
            break
        }
    }
    guard let point = exposedPoint else {
        let top = topWindow(at: CGPoint(x: frame.maxX - 24, y: titleY))
        fputs("Receiver title bar is occluded; receiver=\(windowNumber) frame=\(frame) topPID=\((top?[kCGWindowOwnerPID as String] as? NSNumber)?.intValue ?? 0) topWindow=\((top?[kCGWindowNumber as String] as? NSNumber)?.intValue ?? 0) topBounds=\(String(describing: top.flatMap(windowBounds)))\n", stderr)
        exit(3)
    }
    for type in [CGEventType.leftMouseDown, .leftMouseUp] {
        guard let event = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)
        else { exit(4) }
        event.post(tap: .cghidEventTap)
        Thread.sleep(forTimeInterval: 0.05)
    }
    print("Receiver title-bar activation click sent pid=\(pid) window=\(windowNumber) point=\(point)")
    exit(0)
}
if CommandLine.arguments.count == 3 && CommandLine.arguments[1] == "--send" {
    guard CGPreflightPostEventAccess() else { fputs("Post-event permission unavailable\n", stderr); exit(3) }
    let mode = CommandLine.arguments[2]
    let modifiers: [(CGKeyCode, CGEventFlags)]
    let key: CGKeyCode
    switch mode {
    case "normal": modifiers = [(59, .maskControl), (58, .maskAlternate)]; key = 9
    case "plain": modifiers = [(59, .maskControl), (56, .maskShift)]; key = 9
    case "return": modifiers = []; key = 36
    default: exit(2)
    }
    func post(_ code: CGKeyCode, _ down: Bool, _ flags: CGEventFlags, repeatKey: Bool = false) {
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else { exit(4) }
        event.flags = flags
        event.setIntegerValueField(.keyboardEventAutorepeat, value: repeatKey ? 1 : 0)
        event.post(tap: .cghidEventTap)
        Thread.sleep(forTimeInterval: 0.03)
    }
    var flags: CGEventFlags = []
    for (code, flag) in modifiers { flags.formUnion(flag); post(code, true, flags) }
    post(key, true, flags)
    if mode != "return" { post(key, true, flags, repeatKey: true) }
    post(key, false, flags)
    for (code, flag) in modifiers.reversed() { flags.subtract(flag); post(code, false, flags) }
    exit(0)
}

final class RecordingTextView: NSTextView {
    var pasteCount = 0
    override func paste(_ sender: Any?) { pasteCount += 1; super.paste(sender) }
}

final class ReceiverDelegate: NSObject, NSApplicationDelegate {
    let output: URL
    let payload = ProcessInfo.processInfo.environment["ZSCLIP_VV_RECEIVER_PAYLOAD"] ?? "VV-DELIVERY-PAYLOAD"
    let publishAfter = Double(ProcessInfo.processInfo.environment["ZSCLIP_VV_PUBLISH_AFTER"] ?? "0") ?? 0
    let started = Date()
    var published = false
    var publishedSequence = -1
    var window: NSWindow!
    var editor: RecordingTextView!
    var timer: Timer?
    var eventMonitor: Any?
    var keyDownCount = 0
    var keyUpCount = 0
    var lastKeyCode = -1
    var lastModifiers: UInt = 0
    var keyEvents: [[String: Any]] = []

    init(output: URL) { self.output = output }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let menu = NSMenu()
        let applicationItem = NSMenuItem()
        let applicationMenu = NSMenu()
        applicationMenu.addItem(withTitle: "Quit", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        applicationItem.submenu = applicationMenu
        menu.addItem(applicationItem)
        let editItem = NSMenuItem(title: "Edit", action: nil, keyEquivalent: "")
        let editMenu = NSMenu(title: "Edit")
        for (title, action, key) in [("Cut", #selector(NSText.cut(_:)), "x"),
                                      ("Copy", #selector(NSText.copy(_:)), "c"),
                                      ("Paste", #selector(NSText.paste(_:)), "v"),
                                      ("Select All", #selector(NSText.selectAll(_:)), "a")] {
            editMenu.addItem(withTitle: title, action: action, keyEquivalent: key)
        }
        editItem.submenu = editMenu
        menu.addItem(editItem)
        NSApp.mainMenu = menu
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 320, height: 180),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "ZSClip VV delivery receiver"
        if let area = NSScreen.screens.first?.visibleFrame {
            window.setFrameOrigin(NSPoint(x: max(area.minX, area.maxX - window.frame.width - 12),
                                          y: area.minY + 12))
        }
        let scroller = NSScrollView(frame: NSRect(x: 16, y: 16, width: 288, height: 148))
        scroller.hasVerticalScroller = true
        editor = RecordingTextView(frame: scroller.contentView.bounds)
        editor.isRichText = false
        editor.font = NSFont.systemFont(ofSize: 18)
        editor.string = "LEFT-RIGHT"
        editor.setSelectedRange(NSRange(location: 5, length: 0))
        scroller.documentView = editor
        window.contentView?.addSubview(scroller)
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(editor)
        NSApp.activate(ignoringOtherApps: true)
        eventMonitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .keyUp]) { [weak self] event in
            if event.type == .keyDown { self?.keyDownCount += 1 } else { self?.keyUpCount += 1 }
            self?.lastKeyCode = Int(event.keyCode)
            self?.lastModifiers = event.modifierFlags.rawValue
            self?.keyEvents.append(["phase": event.type == .keyDown ? "down" : "up",
                                    "code": Int(event.keyCode), "modifiers": event.modifierFlags.rawValue,
                                    "repeat": event.type == .keyDown && event.isARepeat])
            return event
        }
        timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            self?.recordState()
        }
        recordState()
    }

    func recordState() {
        if !published && Date().timeIntervalSince(started) >= publishAfter {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(payload, forType: .string)
            if ProcessInfo.processInfo.environment["ZSCLIP_RECEIVER_PUBLISH_HTML"] == "1" {
                NSPasteboard.general.setString("<html><body><b>\(payload)</b></body></html>", forType: .html)
            }
            publishedSequence = NSPasteboard.general.changeCount
            published = true
        }
        let state: [String: Any] = ["pid": ProcessInfo.processInfo.processIdentifier,
                                  "text": editor.string, "payload": payload,
                                  "clipboard_published": published, "key_window": window.isKeyWindow,
                                  "published_sequence": publishedSequence,
                                  "active": NSApp.isActive, "first_responder_is_editor": window.firstResponder === editor,
                                  "selection_location": editor.selectedRange().location,
                                  "selection_length": editor.selectedRange().length,
                                  "frontmost_pid": NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,
                                  "window_number": window.windowNumber,
                                  "titlebar_height": Double(window.frame.height - window.contentRect(forFrameRect: window.frame).height),
                                  "activation_point": ["x": Double(window.frame.maxX - 24),
                                                       "y": Double((NSScreen.screens.first?.frame.maxY ?? 0) - window.frame.maxY + 14)],
                                  "key_down_count": keyDownCount, "key_up_count": keyUpCount,
                                  "last_key_code": lastKeyCode, "last_modifiers": lastModifiers,
                                  "key_events": keyEvents,
                                  "paste_count": editor.pasteCount,
                                  "clipboard_has_html": NSPasteboard.general.data(forType: .html) != nil]
        guard let bytes = try? JSONSerialization.data(withJSONObject: state, options: [.sortedKeys]) else { return }
        try? bytes.write(to: output, options: .atomic)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ application: NSApplication) -> Bool { true }
}

guard CommandLine.arguments.count == 2 else { fatalError("Provide the receiver state JSON path") }
let application = NSApplication.shared
let receiver = ReceiverDelegate(output: URL(fileURLWithPath: CommandLine.arguments[1]))
application.delegate = receiver
application.setActivationPolicy(.regular)
application.run()
