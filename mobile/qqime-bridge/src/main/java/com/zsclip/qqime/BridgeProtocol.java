package com.zsclip.qqime;

import org.json.JSONArray;
import org.json.JSONObject;
import java.io.IOException;
import java.net.InetAddress;
import java.net.URI;
import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;

final class BridgeProtocol {
    static final int MAX_TEXT_CHARS = 262144;
    static final int MAX_TEXT_BYTES = 1048576;
    static final int MAX_RESPONSE_BYTES = 2097152;

    static String endpoint(String raw) throws Exception {
        String value = raw.trim();
        if (value.length() > 512) throw new IllegalArgumentException("电脑地址过长");
        if (value.startsWith("zsclip://pair?")) {
            URI link = new URI(value);
            String host = null;
            for (String field : link.getRawQuery().split("&")) {
                if (field.startsWith("host=")) host = URLDecoder.decode(field.substring(5), "UTF-8");
            }
            if (host == null) throw new IllegalArgumentException("配对链接没有电脑地址");
            value = host;
        }
        if (!value.contains("://")) value = "http://" + value;
        URI uri = new URI(value);
        if (!"http".equalsIgnoreCase(uri.getScheme()) || uri.getUserInfo() != null
                || uri.getQuery() != null || uri.getFragment() != null
                || (uri.getPath() != null && !uri.getPath().isEmpty() && !"/".equals(uri.getPath()))) {
            throw new IllegalArgumentException("请输入电脑局域网 IP 和端口，例如 192.168.1.8:38473");
        }
        String host = uri.getHost();
        if (host == null) throw new IllegalArgumentException("电脑地址格式无效");
        if (host.startsWith("[") && host.endsWith("]")) host = host.substring(1, host.length() - 1);
        if (!host.matches("[0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+")
                && !(host.contains(":") && host.matches("[0-9a-fA-F:]+"))) {
            throw new IllegalArgumentException("请使用局域网 IP 地址");
        }
        InetAddress address = InetAddress.getByName(host);
        host = address.getHostAddress();
        byte[] bytes = address.getAddress();
        boolean privateAddress = address.isSiteLocalAddress() || address.isLinkLocalAddress() || address.isLoopbackAddress();
        if (bytes.length == 16) privateAddress |= (bytes[0] & 0xfe) == 0xfc;
        if (bytes.length == 4) privateAddress |= (bytes[0] & 255) == 100 && (bytes[1] & 192) == 64;
        if (!privateAddress || address.isMulticastAddress() || address.isAnyLocalAddress()) {
            throw new IllegalArgumentException("同步地址必须是局域网或私有 VPN 地址");
        }
        int port = uri.getPort() < 0 ? 38473 : uri.getPort();
        if (port < 1 || port > 65535) throw new IllegalArgumentException("端口必须在 1 至 65535 之间");
        return "http://" + (host.indexOf(':') >= 0 ? "[" + host + "]" : host) + ":" + port;
    }

    static String checkedText(CharSequence input) {
        if (input == null || input.length() == 0) throw new IllegalArgumentException("剪贴板没有文本");
        if (input.length() > MAX_TEXT_CHARS) throw new IllegalArgumentException("文本超过 262144 个字符，已跳过");
        String text = input.toString();
        if (text.trim().isEmpty()) throw new IllegalArgumentException("剪贴板没有文本");
        if (text.getBytes(StandardCharsets.UTF_8).length > MAX_TEXT_BYTES) {
            throw new IllegalArgumentException("文本超过 1 MB，已跳过");
        }
        return text;
    }

    static String signature(String text) {
        try {
            byte[] hash = MessageDigest.getInstance("SHA-256").digest(text.getBytes(StandardCharsets.UTF_8));
            StringBuilder out = new StringBuilder(64);
            for (byte value : hash) out.append(String.format(java.util.Locale.ROOT, "%02x", value & 255));
            return out.toString();
        } catch (Exception e) {
            throw new IllegalStateException("无法计算文本摘要", e);
        }
    }

    static JSONObject pairRequest(String deviceId) throws Exception {
        return new JSONObject().put("device_id", deviceId).put("name", "QQ 输入法 · ZSClip")
                .put("tcp_port", 0).put("capabilities", new JSONArray()
                        .put("text").put("latest").put("client_only").put("pull_only"));
    }

    static JSONObject textEnvelope(String deviceId, String text, long seq) throws Exception {
        int previewEnd = Math.min(80, text.length());
        if (previewEnd > 0 && Character.isHighSurrogate(text.charAt(previewEnd - 1))) previewEnd--;
        return new JSONObject().put("message_id", deviceId + "-" + seq)
                .put("origin_device_id", deviceId).put("origin_seq", seq)
                .put("kind", "text").put("hash", "text:sha256:" + signature(text))
                .put("created_at_ms", seq).put("preview", text.substring(0, previewEnd))
                .put("text", text).put("image_png_base64", JSONObject.NULL).put("file_meta", new JSONArray());
    }

    static JSONObject checkedSendResponse(JSONObject response, boolean automatic) throws IOException {
        if (response == null || !response.optBoolean("ok", false)) {
            throw new IOException("电脑未接受此文本，请重试");
        }
        String ignored = response.optString("ignored");
        if ("direction".equals(ignored)) {
            if (automatic) return response;
            throw new IOException("电脑未接收此条目，请检查电脑同步设置");
        }
        if ("protected".equals(ignored)) {
            throw new IOException("此文本属于电脑受保护内容，电脑未接收");
        }
        if ("self".equals(ignored)) {
            throw new IOException("此条目被识别为电脑自身发出的内容，电脑未重复接收");
        }
        if (!ignored.isEmpty()) throw new IOException("电脑已忽略此文本，未完成发送");
        if (response.has("applied") && !response.optBoolean("applied", false)
                && !response.optBoolean("duplicate", false)) {
            throw new IOException("电脑未接收此文本，请重试");
        }
        return response;
    }

    static boolean alreadyReceived(String stored, String key) {
        if (stored == null || stored.length() > 5000 || key.isEmpty()) return false;
        try {
            JSONArray values = new JSONArray(stored);
            String digest = signature(key);
            for (int i = 0; i < Math.min(64, values.length()); i++) if (digest.equals(values.optString(i))) return true;
        } catch (Exception ignored) { }
        return false;
    }

    static String rememberReceived(String stored, String key) {
        JSONArray next = new JSONArray();
        if (key.isEmpty()) return next.toString();
        String digest = signature(key);
        next.put(digest);
        try {
            JSONArray previous = new JSONArray(stored == null || stored.length() > 5000 ? "[]" : stored);
            for (int i = 0; i < previous.length() && next.length() < 64; i++) {
                String value = previous.optString(i);
                if (value.matches("[0-9a-f]{64}") && !digest.equals(value)) next.put(value);
            }
        } catch (Exception ignored) { }
        return next.toString();
    }

    private BridgeProtocol() { }
}
