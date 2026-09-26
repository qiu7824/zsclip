package com.zsclip.qqime;

import org.json.JSONObject;

public final class SyncModeTest {
    private static int checks;
    private static void check(boolean condition, String label) {
        if (!condition) throw new AssertionError(label); checks++;
    }
    public static void main(String[] args) throws Exception {
        for (String local : SyncMode.VALUES) for (String server : SyncMode.VALUES) {
            String effective = SyncMode.intersection(local, server);
            check(!SyncMode.pushes(effective) || (SyncMode.pushes(local) && SyncMode.pushes(server)), "never adds outbound direction");
            check(!SyncMode.pulls(effective) || (SyncMode.pulls(local) && SyncMode.pulls(server)), "never adds inbound direction");
            check(SyncMode.intersection(local, server).equals(SyncMode.intersection(server, local)), "symmetric restrictions");
        }
        JSONObject policy = new JSONObject().put("protocol", "ZSCLIP_SYNC_POLICY_V1").put("version", 1)
                .put("server_device_id", "pc").put("sync_mode", "bidirectional").put("auto_push", true).put("auto_pull", false);
        check(SyncMode.PHONE_TO_PC.equals(SyncMode.checkedPolicy(policy, "pc")), "server booleans narrow declared direction");
        policy.put("sync_mode", "manual");
        check(SyncMode.MANUAL.equals(SyncMode.checkedPolicy(policy, "pc")), "manual remains manual despite extra flags");
        try { SyncMode.checkedPolicy(policy, "other"); throw new AssertionError("wrong server accepted"); }
        catch (IllegalArgumentException expected) { checks++; }
        check(SyncMode.MANUAL.equals(SyncMode.normalize("unexpected")), "unknown client mode fails closed");
        System.out.println("PASS: " + checks + " synchronization direction checks");
    }
}
