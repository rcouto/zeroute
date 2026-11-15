package org.rcouto.zeroute;

import android.content.Context;
import android.content.Intent;
import android.os.Handler;
import android.os.Looper;
import java.util.Set;
import java.util.concurrent.CopyOnWriteArraySet;
import java.util.concurrent.atomic.AtomicBoolean;

/** Framework dispatch only: command ordering and lifecycle acceptance belong to Rust. */
final class ConnectionController {
  interface Listener {
    void onStatus(ConnectionStatus status);
  }

  private static final Handler MAIN = new Handler(Looper.getMainLooper());
  private static final Set<Listener> LISTENERS = new CopyOnWriteArraySet<>();

  private static final AtomicBoolean PENDING = new AtomicBoolean();
  private static UnderlayObserver underlay;
  private static int observers;

  static synchronized void acquireUnderlay(Context context) {
    if (observers++ == 0) {
      underlay = new UnderlayObserver(context.getApplicationContext());
      underlay.start();
    }
  }

  static synchronized void releaseUnderlay() {
    if (observers > 0 && --observers == 0) {
      underlay.close();
      underlay = null;
    }
  }

  private ConnectionController() {}

  static void subscribe(Listener listener) {
    LISTENERS.add(listener);
    publish();
  }

  static void unsubscribe(Listener listener) {
    LISTENERS.remove(listener);
  }

  static void publish() {
    if (LISTENERS.isEmpty() || !PENDING.compareAndSet(false, true)) return;
    MAIN.post(() -> {
      PENDING.set(false);
      try {
        ConnectionStatus status = ConnectionStatus.read();
        for (Listener listener : LISTENERS) listener.onStatus(status);
      } catch (RuntimeException | LinkageError ignored) {
        /* Each entry point exposes native availability. */
      }
    });
  }

  static void stop(Context context) {
    ConnectionStatus before = ConnectionStatus.read();
    if (before.phase == 4) {
      publish();
      return;
    }
    NativeBridge.sessionEvent(before.token, 3);
    ConnectionStatus after = ConnectionStatus.read();
    if (after.phase == 4)
      context.startService(
          new Intent(context, ZerouteVpnService.class)
              .setAction(ZerouteVpnService.ACTION_STOP)
              .putExtra("token", after.token));
    publish();
  }

  static void connect(Context context) {
    context.startActivity(
        new Intent(context, VpnControlActivity.class).putExtra("connectOnly", true));
  }
}
