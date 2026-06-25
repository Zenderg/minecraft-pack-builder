package com.mpb.runtime;

import java.net.Inet4Address;
import java.net.InetAddress;
import java.net.NetworkInterface;
import java.net.SocketException;
import java.util.Enumeration;

public final class MpbNetwork {
    private MpbNetwork() {}

    public static String displayHostFor(String bindAddress) {
        if (bindAddress == null || bindAddress.isBlank()) {
            return "127.0.0.1";
        }
        if (!"0.0.0.0".equals(bindAddress)) {
            return bindAddress;
        }
        String fallback = null;
        try {
            Enumeration<NetworkInterface> interfaces = NetworkInterface.getNetworkInterfaces();
            while (interfaces != null && interfaces.hasMoreElements()) {
                NetworkInterface networkInterface = interfaces.nextElement();
                if (!networkInterface.isUp() || networkInterface.isLoopback() || networkInterface.isVirtual()) {
                    continue;
                }
                Enumeration<InetAddress> addresses = networkInterface.getInetAddresses();
                while (addresses.hasMoreElements()) {
                    InetAddress address = addresses.nextElement();
                    if (!(address instanceof Inet4Address) || address.isAnyLocalAddress() || address.isLoopbackAddress()) {
                        continue;
                    }
                    String host = address.getHostAddress();
                    if (address.isSiteLocalAddress()) {
                        return host;
                    }
                    if (fallback == null) {
                        fallback = host;
                    }
                }
            }
        } catch (SocketException ignored) {
            // Fall through to localhost; a non-routable wildcard is worse UX than a safe local URL.
        }
        return fallback == null ? "127.0.0.1" : fallback;
    }
}
