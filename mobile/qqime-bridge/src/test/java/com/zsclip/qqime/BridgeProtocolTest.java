package com.zsclip.qqime;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import org.json.JSONObject;

/** JVM contract checks against the ZSClip mobile LAN wire format. */
public final class BridgeProtocolTest {
    private static int checks;

    private static void check(boolean value, String description) {
        if (!value) throw new AssertionError(description);
        checks++;
    }

    private static void rejected(String value) throws Exception {
        try {
            BridgeProtocol.endpoint(value);
            throw new AssertionError("Unsafe address accepted: " + value);
        } catch (IllegalArgumentException | java.net.URISyntaxException | java.net.UnknownHostException expected) {
            checks++;
        }
    }

    private static void rejectedSend(JSONObject response, boolean automatic, String expectedMessage) throws Exception {
        try {
            BridgeProtocol.checkedSendResponse(response, automatic);
            throw new AssertionError("Rejected send response accepted: " + response);
        } catch (java.io.IOException expected) {
            check(expected.getMessage().equals(expectedMessage), "rejected send explains desktop result");
        }
    }

    public static void main(String[] args) throws Exception {
        check(BridgeProtocol.endpoint(" 192.168.1.10 ").equals("http://192.168.1.10:38473"), "default port");
        check(BridgeProtocol.endpoint("http://10.0.0.1:38474/").equals("http://10.0.0.1:38474"), "custom port");
        check(BridgeProtocol.endpoint("zsclip://pair?host=172.16.0.2%3A38475").equals("http://172.16.0.2:38475"), "pair link");
        java.net.URI ipv6 = new java.net.URI(BridgeProtocol.endpoint("[fd00::1]:38473"));
        check(ipv6.getPort() == 38473 && java.net.InetAddress.getByName(ipv6.getHost())
                .equals(java.net.InetAddress.getByName("fd00::1")), "private IPv6");
        check(BridgeProtocol.endpoint("100.64.10.20").equals("http://100.64.10.20:38473"), "private VPN");
        for (String value : new String[] {"https://192.168.1.1", "http://user:pass@192.168.1.1", "192.168.1.1/upload", "8.8.8.8", "224.0.0.1", "0.0.0.0", "100.128.0.1", "192.168.1.1:0", "192.168.1.1:65536", "192.168.1.1?token=x", "192.168.1.1#x", "zsclip://pair?bad=1", "example.com", "[::]"}) rejected(value);
        rejected("abc");
        rejected("dead");

        String text = "  中文\r\nemoji \ud83d\udcce\t\"quoted\"  ";
        check(BridgeProtocol.checkedText(text).equals(text), "text whitespace and Unicode preserved");
        check(BridgeProtocol.signature("abc").equals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"), "SHA-256 known vector");
        JSONObject pair = BridgeProtocol.pairRequest("device-test");
        check(pair.getString("device_id").equals("device-test"), "pair identity");
        check(pair.getInt("tcp_port") == 0, "mobile client does not advertise listener");
        check(pair.getJSONArray("capabilities").toString().contains("client_only"), "desktop skips mobile POST");
        check(pair.getJSONArray("capabilities").toString().contains("pull_only"), "mobile pulls desktop");

        long seq = 1789999999001L;
        JSONObject clip = new JSONObject(new String(BridgeProtocol.textEnvelope("device-test", text, seq)
                .toString().getBytes(StandardCharsets.UTF_8), StandardCharsets.UTF_8));
        check(clip.getString("message_id").equals("device-test-1789999999001"), "message identity");
        check(clip.getString("origin_device_id").equals("device-test"), "origin identity");
        check(clip.getLong("origin_seq") == seq && clip.getLong("created_at_ms") == seq, "sequence and timestamp");
        check(clip.getString("kind").equals("text"), "desktop text decoder");
        check(clip.getString("hash").equals("text:sha256:" + BridgeProtocol.signature(text)), "stable content digest survives relay");
        check(clip.getString("text").equals(text), "JSON round trip preserves text");
        check(clip.isNull("image_png_base64") && clip.getJSONArray("file_meta").length() == 0, "text-only envelope");
        JSONObject direction = new JSONObject().put("ok", true).put("applied", false).put("ignored", "direction");
        check(BridgeProtocol.checkedSendResponse(direction, true) == direction, "automatic direction response remains available for policy handling");
        rejectedSend(direction, false, "电脑未接收此条目，请检查电脑同步设置");
        for (boolean automatic : new boolean[] {false, true}) {
            JSONObject queued = new JSONObject().put("ok", true).put("queued", true);
            check(BridgeProtocol.checkedSendResponse(queued, automatic) == queued, "queued text accepted");
            JSONObject applied = new JSONObject().put("ok", true).put("applied", true);
            check(BridgeProtocol.checkedSendResponse(applied, automatic) == applied, "applied text accepted");
            JSONObject duplicate = new JSONObject().put("ok", true).put("applied", false).put("duplicate", true);
            check(BridgeProtocol.checkedSendResponse(duplicate, automatic) == duplicate, "previously received text remains successful");
            JSONObject legacy = new JSONObject().put("ok", true);
            check(BridgeProtocol.checkedSendResponse(legacy, automatic) == legacy, "legacy success response accepted");
            rejectedSend(new JSONObject().put("ok", true).put("applied", false).put("ignored", "protected"), automatic,
                    "此文本属于电脑受保护内容，电脑未接收");
            rejectedSend(new JSONObject().put("ok", true).put("applied", false).put("ignored", "self"), automatic,
                    "此条目被识别为电脑自身发出的内容，电脑未重复接收");
            rejectedSend(new JSONObject().put("ok", true).put("ignored", "other"), automatic, "电脑已忽略此文本，未完成发送");
            rejectedSend(new JSONObject().put("ok", true).put("applied", false), automatic, "电脑未接收此文本，请重试");
            rejectedSend(new JSONObject().put("ok", false), automatic, "电脑未接受此文本，请重试");
            rejectedSend(new JSONObject(), automatic, "电脑未接受此文本，请重试");
            rejectedSend(null, automatic, "电脑未接受此文本，请重试");
        }
        String ledger = "[]";
        for (int i = 0; i < 100; i++) ledger = BridgeProtocol.rememberReceived(ledger, "remote-" + i);
        check(new org.json.JSONArray(ledger).length() == 64 && ledger.length() < 5000, "receipt storage stays bounded");
        check(BridgeProtocol.alreadyReceived(ledger, "remote-99") && !BridgeProtocol.alreadyReceived(ledger, "remote-0"), "receipt window preserves newest and evicts oldest");
        check(!BridgeProtocol.alreadyReceived("invalid", "remote-99"), "corrupt receipt metadata does not block new content");
        check(!ledger.contains("remote-99"), "receipt storage retains only digests");
        char[] prefix = new char[79];
        Arrays.fill(prefix, 'a');
        String emojiBoundary = new String(prefix) + "\ud83d\udcce";
        String preview = BridgeProtocol.textEnvelope("device-test", emojiBoundary, seq).getString("preview");
        check(preview.length() == 79 && !Character.isHighSurrogate(preview.charAt(preview.length() - 1)), "preview does not split emoji surrogate pair");
        for (String invalid : new String[] {"", " \t\r\n"}) {
            try { BridgeProtocol.checkedText(invalid); throw new AssertionError("blank text accepted"); }
            catch (IllegalArgumentException expected) { checks++; }
        }
        char[] huge = new char[BridgeProtocol.MAX_TEXT_CHARS + 1];
        Arrays.fill(huge, 'x');
        try { BridgeProtocol.checkedText(new String(huge)); throw new AssertionError("oversize text accepted"); }
        catch (IllegalArgumentException expected) { checks++; }
        System.out.println("PASS: " + checks + " LAN protocol checks");
    }
}
