package com.zsclip.qqime;

public final class AndroidLanNetworkTest {
    public static void main(String[] args) throws Exception {
        if (!AndroidLanNetwork.littleEndianAddress(0x2a01a8c0).getHostAddress().equals("192.168.1.42")) throw new AssertionError("DHCP byte order");
        for (int prefix : new int[] {8, 16, 22, 23, 24, 30, 31, 32}) {
            int netmask = Integer.reverseBytes(-1 << (32 - prefix));
            if (AndroidLanNetwork.netmaskPrefix(netmask) != prefix) throw new AssertionError("DHCP mask /" + prefix);
        }
        if (AndroidLanNetwork.netmaskPrefix(0) != -1 || AndroidLanNetwork.netmaskPrefix(0x00ff00ff) != -1) throw new AssertionError("invalid mask");
        System.out.println("PASS: 11 DHCP byte-order and netmask checks");
    }
}
