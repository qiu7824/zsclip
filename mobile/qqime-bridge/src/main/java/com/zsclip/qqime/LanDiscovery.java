package com.zsclip.qqime;

import org.json.JSONObject;
import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.Inet4Address;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.InterfaceAddress;
import java.net.NetworkInterface;
import java.net.SocketTimeoutException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

/** Discovery sends no clipboard data or credentials and never accepts a pairing. */
final class LanDiscovery {
    static final int PORT = 38472;
    static final String MAGIC = "ZSCLIP_LAN_V1";
    static final byte[] PROBE = ("{\"magic\":\"" + MAGIC
            + "\",\"protocol\":1,\"action\":\"discover\"}").getBytes(StandardCharsets.UTF_8);
    static final long SCAN_MILLIS = 5000;

    static final class Device {
        final String id;
        final String name;
        final String endpoint;
        Device(String id, String name, String endpoint) { this.id = id; this.name = name; this.endpoint = endpoint; }
        String address() { return endpoint.substring("http://".length()); }
    }
    interface Listener { void found(Device device); void finished(boolean failed); }
    interface SocketBinder { void bind(DatagramSocket socket) throws IOException; }
    static final class Route {
        final InetAddress local;
        final int prefix;
        final SocketBinder binder;
        Route(InetAddress local, int prefix, SocketBinder binder) {
            this.local = local; this.prefix = prefix; this.binder = binder;
        }
    }
    private static final class Channel {
        final DatagramSocket socket;
        final List<InetAddress> broadcasts;
        final List<InetAddress> hosts;
        int nextHost;
        Channel(DatagramSocket socket, List<InetAddress> broadcasts, List<InetAddress> hosts) {
            this.socket = socket; this.broadcasts = broadcasts; this.hosts = hosts;
        }
    }
    private final int discoveryPort;
    private final List<Route> routes;
    private final String candidate;
    private final boolean broadcastEnabled;
    private volatile boolean cancelled;
    private final List<DatagramSocket> sockets = new ArrayList<DatagramSocket>();
    private volatile int sent;
    private volatile int sendErrors;
    private volatile int received;
    private volatile String networkSummary = "";

    LanDiscovery() { this(PORT); }
    LanDiscovery(int port) { this(port, interfaceRoutes(), ""); }
    LanDiscovery(int port, List<Route> routes, String candidate) {
        this(port, routes, candidate, true);
    }
    LanDiscovery(int port, List<Route> routes, String candidate, boolean broadcastEnabled) {
        if (port < 1 || port > 65535) throw new IllegalArgumentException("discovery port");
        discoveryPort = port;
        this.routes = new ArrayList<Route>(routes);
        this.candidate = candidate;
        this.broadcastEnabled = broadcastEnabled;
    }
    static Device parse(byte[] data, int length, InetAddress source) throws Exception {
        if (length < 2 || length > 4096 || !(source instanceof Inet4Address)) return null;
        JSONObject object = new JSONObject(new String(data, 0, length, StandardCharsets.UTF_8));
        if (!MAGIC.equals(object.optString("magic")) || object.optInt("protocol") != 1) return null;
        int port = object.optInt("tcp_port");
        String id = object.optString("device_id");
        if (port < 1 || port > 65535 || id.isEmpty() || id.length() > 128) return null;
        String endpoint = BridgeProtocol.endpoint(source.getHostAddress() + ":" + port);
        String name = object.optString("name", "ZSClip 电脑").replaceAll("[\\p{Cntrl}\\p{Cf}]", "").trim();
        if (name.isEmpty()) name = "ZSClip 电脑";
        if (name.length() > 80) name = name.substring(0, 80);
        return new Device(id, name, endpoint);
    }
    void start(final Listener listener) {
        new Thread(new Runnable() { public void run() { scan(listener); } }, "ZSClip-discovery").start();
    }
    void cancel() {
        cancelled = true;
        synchronized (sockets) { for (DatagramSocket active : sockets) active.close(); }
    }
    String diagnostics() {
        if (routes.isEmpty()) return "未取得 Wi-Fi 或以太网 IPv4 地址，请先连接局域网。";
        String result = "未发现电脑。手机网络：" + networkSummary + "；已发出 " + sent + " 个探测，收到 " + received + " 个响应";
        if (sendErrors > 0) result += "，" + sendErrors + " 次发送失败";
        return result + "。";
    }
    private void scan(Listener listener) {
        boolean failed = false;
        Set<String> seen = new LinkedHashSet<String>();
        List<Channel> channels = new ArrayList<Channel>();
        try {
            StringBuilder summary = new StringBuilder();
            for (Route route : routes) {
                if (cancelled || channels.size() >= 4) break;
                if (summary.length() > 0) summary.append("、");
                summary.append(route.local.getHostAddress()).append('/').append(route.prefix);
                try { channels.add(channel(route)); }
                catch (IOException unavailable) { sendErrors++; }
            }
            networkSummary = summary.toString();
            if (channels.isEmpty()) { failed = true; return; }
            long start = System.nanoTime();
            long nextBroadcast = 0;
            byte[] bytes = new byte[4097];
            while (!cancelled && (System.nanoTime() - start) < SCAN_MILLIS * 1000000L) {
                long elapsed = System.nanoTime() - start;
                if (broadcastEnabled && elapsed >= nextBroadcast) {
                    for (Channel channel : channels) for (InetAddress target : channel.broadcasts) send(channel.socket, target);
                    nextBroadcast += 1000000000L;
                }
                // Bounded local-subnet probes also work when an access point filters broadcasts.
                for (Channel channel : channels) {
                    for (int i = 0; i < 32 && channel.nextHost < channel.hosts.size() && !cancelled; i++) {
                        send(channel.socket, channel.hosts.get(channel.nextHost++));
                    }
                    if (cancelled) break;
                    // Reset length after unrelated short datagrams so the next reply is not truncated.
                    DatagramPacket packet = new DatagramPacket(bytes, bytes.length);
                    try { channel.socket.receive(packet); }
                    catch (SocketTimeoutException timeout) { continue; }
                    received++;
                    try {
                        Device device = parse(packet.getData(), packet.getLength(), packet.getAddress());
                        if (device != null && seen.size() < 32 && seen.add(device.endpoint) && !cancelled) listener.found(device);
                    } catch (Exception ignored) { /* Malformed datagrams cannot terminate discovery. */ }
                }
            }
            failed = sent == 0;
        } catch (Exception unavailable) { failed = true; }
        finally {
            synchronized (sockets) { for (DatagramSocket active : sockets) active.close(); sockets.clear(); }
            if (!cancelled) listener.finished(failed);
        }
    }
    private Channel channel(Route route) throws IOException {
        DatagramSocket active = new DatagramSocket(null);
        synchronized (sockets) {
            if (cancelled) { active.close(); throw new IOException("cancelled"); }
            sockets.add(active);
        }
        try {
            active.bind(new InetSocketAddress(route == null ? null : route.local, 0));
            if (route != null && route.binder != null) route.binder.bind(active);
            active.setBroadcast(true);
            active.setSoTimeout(20);
            active.setReceiveBufferSize(65536);
            List<InetAddress> broadcasts = new ArrayList<InetAddress>();
            InetAddress directed = route == null ? null : broadcast(route.local, route.prefix);
            if (directed != null) broadcasts.add(directed);
            broadcasts.add(InetAddress.getByName("255.255.255.255"));
            return new Channel(active, broadcasts, route == null ? Collections.<InetAddress>emptyList() : unicastTargets(route, candidate));
        } catch (IOException error) { active.close(); throw error; }
    }
    private void send(DatagramSocket socket, InetAddress target) {
        if (cancelled) return;
        try { socket.send(new DatagramPacket(PROBE, PROBE.length, target, discoveryPort)); sent++; }
        catch (IOException unavailable) { sendErrors++; }
    }
    static boolean usable(InetAddress address) {
        if (!(address instanceof Inet4Address) || address.isLinkLocalAddress() || address.isLoopbackAddress()) return false;
        byte[] bytes = address.getAddress();
        return address.isSiteLocalAddress() || ((bytes[0] & 255) == 100 && (bytes[1] & 192) == 64);
    }
    static long ipv4(InetAddress address) {
        byte[] bytes = address.getAddress();
        return ((bytes[0] & 255L) << 24) | ((bytes[1] & 255L) << 16) | ((bytes[2] & 255L) << 8) | (bytes[3] & 255L);
    }
    static InetAddress ipv4(long value) throws java.net.UnknownHostException {
        return InetAddress.getByAddress(new byte[] {(byte)(value >> 24), (byte)(value >> 16), (byte)(value >> 8), (byte)value});
    }
    static boolean sameSubnet(InetAddress first, InetAddress second, int prefix) {
        if (!(first instanceof Inet4Address) || !(second instanceof Inet4Address) || prefix < 1 || prefix > 32) return false;
        long mask = (0xffffffffL << (32 - prefix)) & 0xffffffffL;
        return (ipv4(first) & mask) == (ipv4(second) & mask);
    }
    static InetAddress broadcast(InetAddress local, int prefix) throws java.net.UnknownHostException {
        if (!(local instanceof Inet4Address) || prefix < 1 || prefix > 30) return null;
        long mask = (0xffffffffL << (32 - prefix)) & 0xffffffffL;
        return ipv4(ipv4(local) | (~mask & 0xffffffffL));
    }
    static List<InetAddress> unicastTargets(Route route, String candidate) throws java.net.UnknownHostException {
        Set<InetAddress> targets = new LinkedHashSet<InetAddress>();
        if (!(route.local instanceof Inet4Address) || route.prefix < 1 || route.prefix > 32) return new ArrayList<InetAddress>(targets);
        try {
            java.net.URI endpoint = new java.net.URI(BridgeProtocol.endpoint(candidate));
            InetAddress preferred = InetAddress.getByName(endpoint.getHost());
            if (sameSubnet(route.local, preferred, route.prefix) && !preferred.equals(route.local)) targets.add(preferred);
        } catch (Exception invalidCandidate) { /* An empty or old candidate does not disable discovery. */ }
        // At most 1,022 hosts per interface; larger subnets use the local /24 plus saved peer.
        int scanPrefix = route.prefix < 22 ? 24 : route.prefix;
        long mask = (0xffffffffL << (32 - scanPrefix)) & 0xffffffffL;
        long first = ipv4(route.local) & mask;
        long last = first | (~mask & 0xffffffffL);
        for (long value = scanPrefix >= 31 ? first : first + 1, end = scanPrefix >= 31 ? last : last - 1; value <= end; value++) {
            InetAddress target = ipv4(value);
            if (!target.equals(route.local)) targets.add(target);
        }
        return new ArrayList<InetAddress>(targets);
    }
    static List<Route> interfaceRoutes() {
        List<Route> result = new ArrayList<Route>();
        try {
            for (NetworkInterface network : Collections.list(NetworkInterface.getNetworkInterfaces())) {
                String name = network.getName().toLowerCase(java.util.Locale.ROOT);
                if (!network.isUp() || network.isLoopback() || network.isVirtual() || network.isPointToPoint()
                        || name.matches("^(tun|tap|ppp|wg|ipsec|rmnet|ccmni).*")) continue;
                for (InterfaceAddress address : network.getInterfaceAddresses()) {
                    if (usable(address.getAddress()) && address.getNetworkPrefixLength() > 0) {
                        result.add(new Route(address.getAddress(), address.getNetworkPrefixLength(), null));
                    }
                }
            }
        } catch (Exception unavailable) { /* Android platform network APIs can supply routes instead. */ }
        return result;
    }
}
