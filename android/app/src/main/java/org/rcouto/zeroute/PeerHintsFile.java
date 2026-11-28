package org.rcouto.zeroute;

import android.util.JsonReader;
import android.util.JsonToken;
import java.io.IOException;
import java.io.StringReader;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Parses the portable JSON envelope and sends bounded ASCII records to Rust. */
final class PeerHintsFile {
  static final int MAX_BYTES = 64 * 1024;
  private static final int MAX_PEERS = 128;
  private static final int MAX_ENDPOINTS = 8;
  private static final Pattern NODE_ID = Pattern.compile("[0-9a-f]{10}");
  private static final Pattern IP_LITERAL = Pattern.compile("[0-9a-fA-F:.]{1,45}");
  private static final Pattern PORT = Pattern.compile("[1-9][0-9]{0,4}");

  // JsonReader permits some invalid string escapes even in strict mode.
  private static final Pattern JSON_STRING_OR_DELIMITER =
      Pattern.compile(
          "\"(?:[^\"\\\\\\x00-\\x1f]|\\\\(?:[\"\\\\/bfnrt]|u[0-9a-fA-F]{4}))*+\"|[\"\\\\]");

  private PeerHintsFile() {}

  static byte[] parse(byte[] file) throws IOException {
    if (file.length > MAX_BYTES) throw invalid();
    String json = StandardCharsets.UTF_8.newDecoder().decode(ByteBuffer.wrap(file)).toString();
    Matcher strings = JSON_STRING_OR_DELIMITER.matcher(json);
    while (strings.find()) {
      if (strings.end() - strings.start() == 1) throw invalid();
    }
    StringBuilder records = new StringBuilder("1\n");
    try (JsonReader reader = new JsonReader(new StringReader(json))) {
      reader.setLenient(false);
      reader.beginObject();
      Set<String> keys = new HashSet<>();
      while (reader.hasNext()) {
        switch (nextKey(reader, keys)) {
          case "version":
            if (!"1".equals(value(reader, JsonToken.NUMBER))) throw invalid();
            break;
          case "peers":
            readPeers(reader, records);
            break;
          default:
            throw invalid();
        }
      }
      reader.endObject();
      if (keys.size() != 2 || reader.peek() != JsonToken.END_DOCUMENT) throw invalid();
    } catch (IllegalStateException | NumberFormatException error) {
      throw invalid();
    }
    return records.toString().getBytes(StandardCharsets.US_ASCII);
  }

  private static void readPeers(JsonReader reader, StringBuilder records) throws IOException {
    reader.beginArray();
    int count = 0;
    while (reader.hasNext()) {
      if (++count > MAX_PEERS) throw invalid();
      reader.beginObject();
      Set<String> keys = new HashSet<>();
      String node = null;
      StringBuilder endpoints = new StringBuilder();
      while (reader.hasNext()) {
        switch (nextKey(reader, keys)) {
          case "nodeId":
            node = value(reader, JsonToken.STRING);
            break;
          case "endpoints":
            readEndpoints(reader, endpoints);
            break;
          default:
            throw invalid();
        }
      }
      reader.endObject();
      if (node == null || !NODE_ID.matcher(node).matches() || endpoints.length() == 0) {
        throw invalid();
      }
      records.append(node).append('\n').append(endpoints);
    }
    reader.endArray();
  }

  private static void readEndpoints(JsonReader reader, StringBuilder endpoints) throws IOException {
    reader.beginArray();
    int count = 0;
    while (reader.hasNext()) {
      if (++count > MAX_ENDPOINTS) throw invalid();
      reader.beginObject();
      Set<String> keys = new HashSet<>();
      String scope = null;
      String address = null;
      String port = null;
      while (reader.hasNext()) {
        switch (nextKey(reader, keys)) {
          case "scope":
            scope = value(reader, JsonToken.STRING);
            break;
          case "ip":
            address = value(reader, JsonToken.STRING);
            break;
          case "port":
            port = value(reader, JsonToken.NUMBER);
            break;
          default:
            throw invalid();
        }
      }
      reader.endObject();
      if ((!"public".equals(scope) && !"lan".equals(scope))
          || address == null
          || !IP_LITERAL.matcher(address).matches()
          || port == null
          || !PORT.matcher(port).matches()) {
        throw invalid();
      }
      endpoints.append(scope).append('\t').append(address).append('\t').append(port).append('\n');
    }
    reader.endArray();
  }

  private static String nextKey(JsonReader reader, Set<String> keys) throws IOException {
    String key = reader.nextName();
    if (!keys.add(key)) throw invalid();
    return key;
  }

  private static String value(JsonReader reader, JsonToken type) throws IOException {
    if (reader.peek() != type) throw invalid();
    return reader.nextString();
  }

  private static IOException invalid() {
    return new IOException("Invalid peer hints file.");
  }
}
