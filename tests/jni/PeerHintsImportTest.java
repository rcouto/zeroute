package org.rcouto.zeroute;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

public final class PeerHintsImportTest {
  private static final String LAN =
      """
      {"version":1,"peers":[{"endpoints":[{"ip":"fd00::1","scope":"lan","port":9993}],
        "nodeId":"0123456789"}]}
      """;
  private static final String DUPLICATE_ENDPOINT =
      LAN.replace(
          "\"port\":9993}",
          "\"port\":9993},{\"ip\":\"fd00:0:0:0:0:0:0:1\",\"port\":9993,\"scope\":\"lan\"}");

  private static void check(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }

  private static boolean importFile(String directory, byte[] file) {
    try {
      return NativeBridge.importPeerHints(directory, PeerHintsFile.parse(file)).isEmpty();
    } catch (IOException error) {
      return false;
    }
  }

  private static byte[] utf8(String input) {
    return input.getBytes(StandardCharsets.UTF_8);
  }

  public static void main(String[] arguments) throws IOException {
    String directory = arguments[0];
    check(NativeBridge.apiVersion() == NativeBridge.API_VERSION, "JNI version mismatch");
    check(NativeBridge.peerHintsCount(directory) == -1, "Expected an empty scratch directory");
    try {
      if (arguments.length == 2) {
        try (InputStream input = Files.newInputStream(Path.of(arguments[1]))) {
          check(
              importFile(directory, input.readNBytes(PeerHintsFile.MAX_BYTES + 1)),
              "Peer hints import rejected");
        }
        System.out.println("Imported public peer count: " + NativeBridge.peerHintsCount(directory));
        return;
      }
      check(importFile(directory, utf8(LAN)), "Valid LAN hints were rejected");
      check(NativeBridge.peerHintsCount(directory) == 0, "LAN hints became eligible");
      for (String invalid :
          new String[] {
            LAN.replace("\"lan\"", "\"public\""),
            DUPLICATE_ENDPOINT,
            LAN.replace("\"version\":1", "\"version\":1,\"version\":1"),
            LAN.replace("\"version\":1", "\"version\":\"1\""),
            LAN.replace("\"version\":1", "\"version\":01"),
            LAN.replace("9993", "9993.0"),
            LAN.replace("9993", "65536"),
            LAN.replace("fd00::1", "fd00::1%wlan0"),
            LAN.replace("lan", "\\lan"),
            LAN + "{}",
          }) {
        check(!importFile(directory, utf8(invalid)), "Invalid hints were accepted");
        check(NativeBridge.peerHintsCount(directory) == 0, "Invalid hints replaced saved data");
      }
      check(
          !importFile(directory, new byte[PeerHintsFile.MAX_BYTES + 1]), "Oversized file accepted");
      check(!importFile(directory, new byte[] {(byte) 0xff}), "Invalid UTF-8 accepted");
      System.out.println("Peer hints import regression checks passed");
    } finally {
      check(NativeBridge.removePeerHints(directory).isEmpty(), "Peer hints cleanup failed");
    }
  }
}
