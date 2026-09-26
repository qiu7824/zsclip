package com.zsclip.qqime;

import java.net.InetAddress;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

public final class LanDiscoveryTest {
    private static int checks;
    private static void check(boolean result) {
        if (!result) throw new AssertionError("discovery check " + (checks + 1));
        checks++;
    }
    private static LanDiscovery.Device parse(JSONObject packet, String source) throws Exception {
        byte[] bytes = packet.toString().getBytes(StandardCharsets.UTF_8);
        return LanDiscovery.parse(bytes, bytes.length, InetAddress.getByName(source));
    }
    public static void main(String[] args) throws Exception {
        JSONObject probe = new JSONObject(new String(LanDiscovery.PROBE, StandardCharsets.UTF_8));
        check(probe.getString("action").equals("discover"));
        JSONObject packet = new JSONObject().put("magic", LanDiscovery.MAGIC).put("protocol", 1)
                .put("device_id", "pc-test").put("name", "电脑\n一\u202e")
                .put("tcp_port", 38475).put("host", "evil.example.com");
        LanDiscovery.Device device = parse(packet, "192.168.2.5");
        check(device.endpoint.equals("http://192.168.2.5:38475"));
        check(device.name.equals("电脑一"));
        check(device.address().equals("192.168.2.5:38475"));
        check(parse(packet.put("protocol", 2), "192.168.2.5") == null);
        check(parse(packet.put("protocol", 1).put("magic", "other"), "192.168.2.5") == null);
        packet.put("magic", LanDiscovery.MAGIC);
        for (int port : new int[] {0, -1, 65536}) check(parse(packet.put("tcp_port", port), "192.168.2.5") == null);
        packet.put("tcp_port", 38473);
        check(parse(packet.put("device_id", ""), "192.168.2.5") == null);
        packet.put("device_id", "pc-test");
        check(parse(packet, "fd00::1") == null);
        try { parse(packet, "8.8.8.8"); throw new AssertionError("public source accepted"); }
        catch (IllegalArgumentException expected) { checks++; }
        check(LanDiscovery.parse(new byte[4097], 4097, InetAddress.getByName("192.168.2.5")) == null);
        check(parse(packet.put("name", ""), "192.168.2.5").name.equals("ZSClip 电脑"));
        check(LanDiscovery.broadcast(InetAddress.getByName("192.168.1.42"), 23).getHostAddress().equals("192.168.1.255"));
        check(LanDiscovery.broadcast(InetAddress.getByName("10.12.3.4"), 16).getHostAddress().equals("10.12.255.255"));
        check(LanDiscovery.broadcast(InetAddress.getByName("192.168.1.42"), 32) == null);
        check(!LanDiscovery.usable(InetAddress.getByName("169.254.7.8")));
        check(!LanDiscovery.usable(InetAddress.getByName("127.0.0.1")));
        check(!LanDiscovery.usable(InetAddress.getByName("8.8.8.8")));
        check(LanDiscovery.usable(InetAddress.getByName("192.168.1.42")));
        LanDiscovery.Route route = new LanDiscovery.Route(InetAddress.getByName("192.168.1.88"), 24, null);
        java.util.List<InetAddress> hosts = LanDiscovery.unicastTargets(route, "192.168.1.42");
        check(hosts.size() == 253 && hosts.get(0).getHostAddress().equals("192.168.1.42"));
        check(!hosts.contains(route.local) && !hosts.contains(InetAddress.getByName("192.168.1.255")));
        check(!LanDiscovery.unicastTargets(route, "192.168.2.42").contains(InetAddress.getByName("192.168.2.42")));
        route = new LanDiscovery.Route(InetAddress.getByName("10.1.2.88"), 16, null);
        hosts = LanDiscovery.unicastTargets(route, "10.1.3.42");
        check(hosts.size() == 254 && hosts.get(0).getHostAddress().equals("10.1.3.42"));
        check(!hosts.contains(InetAddress.getByName("10.1.4.42")));
        route = new LanDiscovery.Route(InetAddress.getByName("10.1.2.88"), 22, null);
        check(LanDiscovery.unicastTargets(route, "").size() == 1021);
        check(LanDiscovery.SCAN_MILLIS < 6000);
        final java.util.concurrent.CountDownLatch stopped = new java.util.concurrent.CountDownLatch(1);
        LanDiscovery scan = new LanDiscovery();
        scan.cancel();
        scan.start(new LanDiscovery.Listener() {
            public void found(LanDiscovery.Device ignored) { throw new AssertionError("cancelled callback"); }
            public void finished(boolean ignored) { stopped.countDown(); }
        });
        check(!stopped.await(300, java.util.concurrent.TimeUnit.MILLISECONDS));
        System.out.println("PASS: " + checks + " discovery validation/cancellation checks");
    }
}
