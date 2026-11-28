# ZeroTier peer endpoint hints

With Zeroute disconnected, open **Details** and choose **Import or replace
peer hints**. Select a JSON file from a source you trust. Zeroute shows how
many peers have usable public hints. The next connection uses the saved
file. **Remove peer hints** clears the imported candidates. Zeroute stores
the validated public candidates in app-private storage excluded from backup.

Version 1 expects an object with integer `version` set to `1` and a `peers`
array. Each peer has a unique lowercase ten-digit hexadecimal `nodeId` and
one to eight ordered `endpoints`. Each endpoint has a `scope` of `public` or
`lan`, an `ip` containing a bare IPv4 or IPv6 literal without a zone ID,
and an integer UDP `port` from 1 to 65535. Duplicate IP and port pairs for
the same peer are rejected after parsing the addresses. The file can contain
up to 128 peers and 64 KiB of UTF-8 JSON.

Use globally reachable unicast addresses for public hints. Zeroute validates
LAN entries and leaves them inactive on every network in version 1. Changes to
peer hints take effect on the next connection. An invalid import preserves
the last saved file. If the saved file becomes unreadable, Zeroute still
connects using ordinary discovery.

The ZeroTier core can ask for a hint while retrying a known peer. Zeroute
offers one eligible endpoint at a time, beginning in the supplied order.
ZeroTier still discovers peer identities and authenticates connections. Hints
may shorten direct path establishment. DNS and first-time identity discovery
still need their normal network paths.
