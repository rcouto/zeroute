package org.rcouto.zeroute;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.LinkProperties;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import java.util.HashMap;
import java.util.Map;

/** Reports non-VPN network facts; availability is not proof of overlay reachability. */
final class UnderlayObserver implements AutoCloseable {
  private final ConnectivityManager manager;
  private final Map<Network, NetworkCapabilities> networks = new HashMap<>();
  private boolean registered;
  private long revision;
  private final Map<Network, LinkProperties> properties = new HashMap<>();
  private final ConnectivityManager.NetworkCallback callback =
      new ConnectivityManager.NetworkCallback() {
        @Override
        public void onCapabilitiesChanged(Network network, NetworkCapabilities capabilities) {
          synchronized (UnderlayObserver.this) {
            if (!registered) return;
            if (!networks.containsKey(network)) revision++;
            networks.put(network, capabilities);
            report();
          }
        }

        @Override
        public void onLinkPropertiesChanged(Network network, LinkProperties value) {
          synchronized (UnderlayObserver.this) {
            if (!registered) return;
            LinkProperties old = properties.put(network, value);
            if (old != null && !old.equals(value)) {
              revision++;
              report();
            }
          }
        }

        @Override
        public void onLost(Network network) {
          synchronized (UnderlayObserver.this) {
            if (!registered) return;
            if (networks.remove(network) != null) revision++;
            properties.remove(network);
            report();
          }
        }
      };

  UnderlayObserver(Context context) {
    manager = context.getSystemService(ConnectivityManager.class);
  }

  @SuppressWarnings("deprecation")
  synchronized void start() {
    if (registered) return;
    try {
      registered = true;
      manager.registerNetworkCallback(
          new NetworkRequest.Builder()
              .addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN)
              .build(),
          callback);
      for (Network network : manager.getAllNetworks()) {
        NetworkCapabilities caps = manager.getNetworkCapabilities(network);
        if (caps != null && !caps.hasTransport(NetworkCapabilities.TRANSPORT_VPN)) {
          networks.put(network, caps);
          revision++;
        }
      }
      report();
    } catch (RuntimeException error) {
      if (registered) {
        try {
          manager.unregisterNetworkCallback(callback);
        } catch (RuntimeException ignored) {
          /* Registration may have failed. */
        }
      }
      registered = false;
      NativeBridge.networkFacts(0, revision);
    }
  }

  private void report() {
    int flags = 1;
    for (NetworkCapabilities caps : networks.values()) {
      if (caps.hasTransport(NetworkCapabilities.TRANSPORT_VPN)) continue;
      flags |= 2;
      if (caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)) flags |= 4;
      if (caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_CAPTIVE_PORTAL)) flags |= 8;
      if (caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) flags |= 16;
      if (caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)) flags |= 32;
      if (caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)) flags |= 64;
      if (!caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)) flags |= 128;
    }
    NativeBridge.networkFacts(flags, revision);
    ConnectionController.publish();
  }

  @Override
  public synchronized void close() {
    if (registered) {
      registered = false;
      try {
        manager.unregisterNetworkCallback(callback);
      } catch (RuntimeException ignored) {
        /* Already released. */
      }
    }
    networks.clear();
    properties.clear();
    NativeBridge.networkFacts(0, revision);
  }
}
