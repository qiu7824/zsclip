package com.zsclip.qqime;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.DhcpInfo;
import android.net.LinkAddress;
import android.net.LinkProperties;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.wifi.WifiManager;
import java.io.IOException;
import java.net.DatagramSocket;
import java.net.HttpURLConnection;
import java.net.InetAddress;
import java.net.URL;
import java.util.ArrayList;
import java.util.List;

/** Use the local network even if Android uses cellular data as its default network. */
final class AndroidLanNetwork {
    private static final class BoundRoute {
        final LanDiscovery.Route route;
        final Network network;
        BoundRoute(LanDiscovery.Route route, Network network) { this.route = route; this.network = network; }
    }
    static LanDiscovery discovery(Context context, String candidate) {
        List<LanDiscovery.Route> routes = new ArrayList<LanDiscovery.Route>();
        for (BoundRoute entry : routes(context)) routes.add(entry.route);
        if (routes.isEmpty()) routes.addAll(LanDiscovery.interfaceRoutes());
        return new LanDiscovery(LanDiscovery.PORT, routes, candidate);
    }
    static HttpURLConnection openConnection(Context context, URL url) throws IOException {
        InetAddress host = InetAddress.getByName(url.getHost());
        BoundRoute best = null;
        for (BoundRoute entry : routes(context)) {
            if (entry.network != null && LanDiscovery.sameSubnet(entry.route.local, host, entry.route.prefix)
                    && (best == null || entry.route.prefix > best.route.prefix)) best = entry;
        }
        return (HttpURLConnection)(best == null ? url.openConnection() : best.network.openConnection(url));
    }
    private static List<BoundRoute> routes(Context context) {
        List<BoundRoute> result = new ArrayList<BoundRoute>();
        try {
            ConnectivityManager manager = (ConnectivityManager)context.getSystemService(Context.CONNECTIVITY_SERVICE);
            if (manager != null) for (final Network network : manager.getAllNetworks()) {
                NetworkCapabilities capabilities = manager.getNetworkCapabilities(network);
                if (capabilities == null || capabilities.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
                        || (!capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)
                        && !capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET))) continue;
                LinkProperties properties = manager.getLinkProperties(network);
                if (properties == null) continue;
                for (LinkAddress address : properties.getLinkAddresses()) {
                    if (!LanDiscovery.usable(address.getAddress()) || address.getPrefixLength() < 1) continue;
                    add(result, new LanDiscovery.Route(address.getAddress(), address.getPrefixLength(),
                            new LanDiscovery.SocketBinder() { public void bind(DatagramSocket socket) throws IOException {
                                network.bindSocket(socket);
                            }}), network);
                }
            }
        } catch (Exception unavailable) { /* Older Android builds can still expose DHCP information. */ }
        try {
            WifiManager wifi = (WifiManager)context.getApplicationContext().getSystemService(Context.WIFI_SERVICE);
            DhcpInfo dhcp = wifi == null || !wifi.isWifiEnabled() ? null : wifi.getDhcpInfo();
            if (dhcp != null) {
                InetAddress local = littleEndianAddress(dhcp.ipAddress);
                int prefix = netmaskPrefix(dhcp.netmask);
                java.net.NetworkInterface adapter = java.net.NetworkInterface.getByInetAddress(local);
                if (LanDiscovery.usable(local) && prefix > 0 && adapter != null && adapter.isUp()
                        && !adapter.isLoopback() && !adapter.isVirtual() && !adapter.isPointToPoint()) {
                    add(result, new LanDiscovery.Route(local, prefix, null), null);
                }
            }
        } catch (Exception unavailable) { /* Missing DHCP data must not discard platform routes. */ }
        return result;
    }
    private static void add(List<BoundRoute> entries, LanDiscovery.Route route, Network network) {
        for (BoundRoute entry : entries) if (entry.route.local.equals(route.local)) return;
        entries.add(new BoundRoute(route, network));
    }
    static InetAddress littleEndianAddress(int value) throws java.net.UnknownHostException {
        return InetAddress.getByAddress(new byte[] {(byte)value, (byte)(value >> 8), (byte)(value >> 16), (byte)(value >> 24)});
    }
    static int netmaskPrefix(int littleEndianMask) {
        int mask = Integer.reverseBytes(littleEndianMask);
        int prefix = Integer.bitCount(mask);
        return prefix > 0 && mask == (-1 << (32 - prefix)) ? prefix : -1;
    }
    private AndroidLanNetwork() { }
}
