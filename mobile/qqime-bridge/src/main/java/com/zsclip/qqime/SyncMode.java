package com.zsclip.qqime;

import org.json.JSONObject;

/** Explicit client directions; server policy can only reduce automatic access. */
final class SyncMode {
    static final String MANUAL = "manual";
    static final String PHONE_TO_PC = "phone_to_pc";
    static final String PC_TO_PHONE = "pc_to_phone";
    static final String BOTH = "bidirectional";
    static final String[] VALUES = { MANUAL, PHONE_TO_PC, PC_TO_PHONE, BOTH };
    static final String[] LABELS = { "仅手动发送 / 接收", "手机 → 电脑", "电脑 → 手机", "双向自动同步" };

    static String normalize(String value) {
        for (String mode : VALUES) if (mode.equals(value)) return mode;
        return MANUAL;
    }
    static boolean pushes(String mode) { return PHONE_TO_PC.equals(mode) || BOTH.equals(mode); }
    static boolean pulls(String mode) { return PC_TO_PHONE.equals(mode) || BOTH.equals(mode); }
    static int index(String mode) {
        for (int i = 0; i < VALUES.length; i++) if (VALUES[i].equals(mode)) return i;
        return 0;
    }
    static String label(String mode) { return LABELS[index(mode)]; }
    static String intersection(String client, String server) {
        boolean push = pushes(client) && pushes(server), pull = pulls(client) && pulls(server);
        return push ? (pull ? BOTH : PHONE_TO_PC) : (pull ? PC_TO_PHONE : MANUAL);
    }
    static String checkedPolicy(JSONObject policy, String expectedDevice) throws Exception {
        if (!"ZSCLIP_SYNC_POLICY_V1".equals(policy.optString("protocol")) || policy.optInt("version") != 1
                || !expectedDevice.equals(policy.optString("server_device_id"))) {
            throw new IllegalArgumentException("电脑同步策略无效，请重新连接");
        }
        String mode = normalize(policy.optString("sync_mode"));
        boolean push = pushes(mode) && policy.optBoolean("auto_push", false);
        boolean pull = pulls(mode) && policy.optBoolean("auto_pull", false);
        return push ? (pull ? BOTH : PHONE_TO_PC) : (pull ? PC_TO_PHONE : MANUAL);
    }
    private SyncMode() { }
}
