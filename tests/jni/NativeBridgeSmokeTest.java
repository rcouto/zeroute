package org.rcouto.zeroute;

public final class NativeBridgeSmokeTest {
  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }

  public static void main(String[] arguments) {
    String directory = arguments[0];
    check(NativeBridge.apiVersion() == NativeBridge.API_VERSION, "JNI version mismatch");
    check(NativeBridge.readiness() == 0, "Fresh process must be disconnected");
    check(
        (NativeBridge.capabilities() & ~NativeBridge.CAPABILITY_ENGINE) == 0, "Unknown capability");
    check(NativeBridge.loadNetworkId(directory).isEmpty(), "Unexpected existing settings");
    check(!NativeBridge.saveNetworkId(directory, "bad").isEmpty(), "Invalid ID accepted");
    check(NativeBridge.saveNetworkId(directory, "FEDCBA9876543210").isEmpty(), "Save failed");
    check(
        NativeBridge.loadNetworkId(directory).equals("fedcba9876543210"),
        "JNI string roundtrip failed");
    check(!NativeBridge.saveNetworkId(directory, "0000000000000000").isEmpty(), "Zero ID accepted");
    check(
        NativeBridge.loadNetworkId(directory).equals("fedcba9876543210"),
        "Invalid input replaced settings");
    check(NativeBridge.readiness() == 0, "Saving settings must not claim connectivity");
    check(NativeBridge.engineVersion().isEmpty(), "Host must not claim an Android backend");
    check(!NativeBridge.startEngine(directory).isEmpty(), "Host must reject engine start");
    check(NativeBridge.stopEngine().isEmpty(), "Stopped engine must stop idempotently");
    check(!NativeBridge.controlPlaneOnline(), "Stopped engine must not claim root connectivity");
    check(NativeBridge.nodeAddress().isEmpty(), "Stopped host must not invent a node address");
    check(
        NativeBridge.vpnConfiguration().isEmpty(), "Stopped host must not expose tunnel settings");
    check(!NativeBridge.attachTunnel(-1, 0).isEmpty(), "Invalid tunnel attachment accepted");
    check(NativeBridge.trafficDiagnostics().isEmpty(), "Host must not invent tunnel traffic");
    long old = NativeBridge.beginSession();
    check(old > 0 && NativeBridge.beginSession() == 0, "Duplicate consent attempt accepted");
    check(
        !NativeBridge.saveNetworkId(directory, "0123456789abcdef").isEmpty(),
        "Settings changed during consent");
    check(NativeBridge.sessionEvent(old, 3), "Pending consent could not be cancelled");
    check(!NativeBridge.sessionEvent(old, 1), "Cancelled consent started a service");
    long current = NativeBridge.beginSession();
    check(!NativeBridge.sessionEvent(old, 1), "Stale consent accepted for new attempt");
    check(NativeBridge.sessionEvent(current, 1), "Service handoff rejected");
    check(NativeBridge.sessionEvent(current, 3), "Startup could not be cancelled");
    check(NativeBridge.beginSession() == 0, "Connect accepted before cleanup acknowledgement");
    check(NativeBridge.sessionEvent(current, 4), "Cleanup acknowledgement rejected");
    check(
        NativeBridge.statusSnapshot().split("\n", -1).length == 3, "Snapshot wire format mismatch");
    check(NativeBridge.sessionAccepts(current, 0), "Stopped lifecycle not published");
    System.out.println("JNI settings/readiness/lifecycle smoke test passed");
  }
}
