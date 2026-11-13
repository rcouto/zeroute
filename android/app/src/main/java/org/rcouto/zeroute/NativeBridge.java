package org.rcouto.zeroute;

final class NativeBridge {
  static {
    System.loadLibrary("zeroute_core");
  }

  private NativeBridge() {}

  static native int apiVersion();
}
