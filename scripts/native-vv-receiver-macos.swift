import AppKit
import Foundation

final class ReceiverDelegate: NSObject, NSApplicationDelegate {
    let output: URL
    let payload = ProcessInfo.processInfo.environment["ZSCLIP_VV_RECEIVER_PAYLOAD"] ?? "VV-DELIVERY-PAYLOAD"
    let publishAfter = Double(ProcessInfo.processInfo.environment["ZSCLIP_VV_PUBLISH_AFTER"] ?? "0") ?? 0
    let started = Date()
    var published = false
    var window: NSWindow!
    var editor: NSTextView!
    var timer: Timer?

    init(output: URL) { self.output = output }

    func applicationDidFinishLaunching(_ notification: Notification) {
        window = NSWindow(contentRect: NSRect(x: 80, y: 160, width: 640, height: 260),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "ZSClip VV delivery receiver"
        let scroller = NSScrollView(frame: NSRect(x: 16, y: 16, width: 608, height: 228))
        scroller.hasVerticalScroller = true
        editor = NSTextView(frame: scroller.contentView.bounds)
        editor.isRichText = false
        editor.font = NSFont.systemFont(ofSize: 18)
        editor.string = "LEFT-RIGHT"
        editor.setSelectedRange(NSRange(location: 5, length: 0))
        scroller.documentView = editor
        window.contentView?.addSubview(scroller)
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(editor)
        NSApp.activate(ignoringOtherApps: true)
        timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            self?.recordState()
        }
        recordState()
    }

    func recordState() {
        if !published && Date().timeIntervalSince(started) >= publishAfter {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(payload, forType: .string)
            published = true
        }
        let state: [String: Any] = ["pid": ProcessInfo.processInfo.processIdentifier,
                                  "text": editor.string, "payload": payload,
                                  "clipboard_published": published, "key_window": window.isKeyWindow]
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
