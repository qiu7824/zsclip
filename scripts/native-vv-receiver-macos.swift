import AppKit
import Foundation

// Test driver posts normal system input. It never invokes a ZSClip callback or
// the receiver's paste action; the receiver records what AppKit actually did.
if CommandLine.arguments.count == 3 && CommandLine.arguments[1] == "--activate" {
    guard let pid = Int32(CommandLine.arguments[2]),
          let app = NSRunningApplication(processIdentifier: pid),
          app.activate(options: [.activateIgnoringOtherApps]) else { exit(2) }
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
        window = NSWindow(contentRect: NSRect(x: 80, y: 160, width: 640, height: 260),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "ZSClip VV delivery receiver"
        let scroller = NSScrollView(frame: NSRect(x: 16, y: 16, width: 608, height: 228))
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
