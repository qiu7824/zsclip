package com.zsclip.qqime;

import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.concurrent.*;
import java.util.concurrent.atomic.*;
import org.json.JSONObject;

/** Local Windows/JVM transport test; no Android installation or mobile runtime is used. */
public final class LanDiscoverySocketTest {
    public static void main(String[] args) throws Exception {
        final AtomicInteger probes = new AtomicInteger(), found = new AtomicInteger();
        final AtomicReference<Throwable> failure = new AtomicReference<Throwable>();
        final CountDownLatch done = new CountDownLatch(1);
        try (final DatagramSocket server = new DatagramSocket(new InetSocketAddress("127.0.0.1", 0))) {
            server.setSoTimeout(6500);
            Thread responder = new Thread(new Runnable() { public void run() { try {
                byte[] bytes = new byte[4096];
                while (!server.isClosed()) {
                    DatagramPacket request = new DatagramPacket(bytes, bytes.length);
                    server.receive(request);
                    JSONObject json = new JSONObject(new String(request.getData(), 0, request.getLength(), StandardCharsets.UTF_8));
                    if (!json.optString("action").equals("discover")) throw new AssertionError("unexpected operation");
                    if (request.getPort() == server.getLocalPort()) throw new AssertionError("source must use ephemeral port");
                    probes.incrementAndGet();
                    byte[] shortNoise = new byte[] {0};
                    server.send(new DatagramPacket(shortNoise, shortNoise.length, request.getAddress(), request.getPort()));
                    byte[] reply = new JSONObject().put("magic", LanDiscovery.MAGIC).put("protocol", 1)
                            .put("device_id", "synthetic-pc").put("name", "Synthetic LAN PC").put("tcp_port", 38499)
                            .put("host", "8.8.8.8").toString().getBytes(StandardCharsets.UTF_8);
                    for (int i = 0; i < 2; i++) server.send(new DatagramPacket(reply, reply.length, request.getAddress(), request.getPort()));
                }
            } catch (Throwable error) { if (!server.isClosed()) failure.set(error); } }});
            responder.start();
            // Suppress broadcast sends to model an AP that drops them, keeping real UDP unicast.
            LanDiscovery.Route route = new LanDiscovery.Route(InetAddress.getByName("127.0.0.2"), 30, null);
            LanDiscovery scan = new LanDiscovery(server.getLocalPort(), Collections.singletonList(route), "", false);
            long started = System.nanoTime();
            scan.start(new LanDiscovery.Listener() {
                public void found(LanDiscovery.Device device) {
                    if (!device.id.equals("synthetic-pc") || !device.endpoint.equals("http://127.0.0.1:38499")) failure.set(new AssertionError("incorrect reply"));
                    found.incrementAndGet();
                }
                public void finished(boolean failed) { if (failed) failure.set(new AssertionError("scan failure")); done.countDown(); }
            });
            if (!done.await(6, TimeUnit.SECONDS)) throw new AssertionError("scan deadline");
            long elapsed = (System.nanoTime() - started) / 1000000;
            server.close(); responder.join(1000); scan.cancel();
            if (failure.get() != null) throw new AssertionError(failure.get());
            if (probes.get() != 1 || found.get() != 1) throw new AssertionError("unicast fallback or duplicate suppression: " + probes + "/" + found);
            if (elapsed >= 6000) throw new AssertionError("unbounded scan");
            System.out.println("PASS: broadcast-disabled unicast discovery, request-source ephemeral reply, short-before-long receive, source-address endpoint, deduplication; " + elapsed + " ms");
        }
    }
}
