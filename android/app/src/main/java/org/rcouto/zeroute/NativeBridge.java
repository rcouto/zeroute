package org.rcouto.zeroute;

final class NativeBridge {
  static final int API_VERSION = 4;
  static final int CAPABILITY_ENGINE = 1;

  static {
    System.loadLibrary("zeroute_core");
  }

  private NativeBridge() {}

  static native int apiVersion();

  static native int capabilities();

  static native String engineVersion();

  static native String startEngine(String directory);

  static native String stopEngine();

  // Object keeps this bridge usable in host JNI tests without Android classes.
  static native String startVpnEngine(String directory, Object service);

  static native String vpnConfiguration();

  static native String attachTunnel(int fd, long revision);

  static native void vpnFailed();

  static native String nodeAddress();

  static native boolean controlPlaneOnline();

  static native String engineError();

  static native String trafficDiagnostics();

  static native long readiness();

  static native String loadNetworkId(String directory);

  // Empty means saved; validation/storage failures return a user-facing message.
  static native String saveNetworkId(String directory, String networkId);
}
