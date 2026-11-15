package org.rcouto.zeroute;

/** One Rust-owned snapshot shared by every Android presentation surface. */
final class ConnectionStatus {
  final long token;
  final int phase;
  final int state;
  final int stage;
  final boolean dnsConfigured;
  final boolean roots;
  final int network;
  final String node;
  final String error;

  private ConnectionStatus(String wire) {
    String[] lines = wire.split("\n", -1);
    if (lines.length != 3) throw new IllegalStateException("Invalid snapshot");
    String[] fields = lines[0].split(" ");
    if (fields.length != 7) throw new IllegalStateException("Invalid snapshot header");
    token = Long.parseLong(fields[0]);
    phase = Integer.parseInt(fields[1]);
    state = Integer.parseInt(fields[2]);
    stage = Integer.parseInt(fields[3]);
    dnsConfigured = fields[4].equals("1");
    roots = fields[5].equals("1");
    network = Integer.parseInt(fields[6]);
    node = lines[1].equals("0000000000") ? "" : lines[1];
    error = lines[2];
    if (phase < 0 || phase > 5 || state < 0 || state > 13 || stage < 0 || stage > 5)
      throw new IllegalStateException("Unknown snapshot state");
  }

  static ConnectionStatus read() {
    if (NativeBridge.apiVersion() != NativeBridge.API_VERSION)
      throw new IllegalStateException("Native ABI mismatch");
    return new ConnectionStatus(NativeBridge.statusSnapshot());
  }

  boolean active() {
    return phase >= 1 && phase <= 4;
  }

  int title() {
    return new int[] {
          R.string.state_stopped,
          R.string.state_permission,
          R.string.state_engine,
          R.string.state_configuration,
          R.string.state_tunnel,
          R.string.state_dns,
          R.string.state_ready,
          R.string.state_ip_only,
          R.string.state_admission,
          R.string.state_offline,
          R.string.state_reconnecting,
          R.string.state_portal,
          R.string.state_failed,
          R.string.state_stopping
        }
        [state];
  }

  int description() {
    if (state == 0) return R.string.description_off;
    if (state == 6 || state == 7) return R.string.description_connected;
    if (state == 5) return R.string.description_dns;
    if (state == 8) return R.string.description_admission;
    if (state == 9) return R.string.description_offline;
    if (state == 11) return R.string.description_portal;
    if (state == 13) return R.string.description_stopping;
    if (state == 12) return R.string.description_failed;
    if (state == 1) return R.string.description_permission;
    return R.string.description_connecting;
  }
}
