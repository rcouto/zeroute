package org.rcouto.zeroute;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.net.VpnService;
import android.os.Build;
import android.os.ParcelFileDescriptor;
import android.system.OsConstants;
import java.io.IOException;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;

/** Android owns permission, foreground lifetime and TUN creation; Rust owns forwarding. */
public final class ZerouteVpnService extends VpnService {
  static final String ACTION_CONNECT = "org.rcouto.zeroute.CONNECT";
  static final String ACTION_STOP = "org.rcouto.zeroute.STOP";
  private static final String CHANNEL = "vpn";
  private final ScheduledExecutorService worker = Executors.newSingleThreadScheduledExecutor();
  private ScheduledFuture<?> monitor;
  private ParcelFileDescriptor tunnel;
  private String appliedConfiguration = "";
  private volatile boolean closing;
  private boolean failed;
  private boolean cleaned;

  // Called synchronously by JNI before its transport worker starts.
  public boolean protectSocket(int descriptor) {
    return protect(descriptor);
  }

  @Override
  public int onStartCommand(Intent intent, int flags, int startId) {
    if (intent == null || ACTION_STOP.equals(intent.getAction())) {
      requestStop(false);
      return START_NOT_STICKY;
    }
    if (!ACTION_CONNECT.equals(intent.getAction()) || prepare(this) != null) {
      requestStop(false);
      return START_NOT_STICKY;
    }
    showForegroundNotification();
    worker.execute(() -> {
      if (closing || monitor != null) return;
      try {
        NativeBridge.stopEngine();
        String error = NativeBridge.startVpnEngine(getNoBackupFilesDir().getAbsolutePath(), this);
        if (!error.isEmpty()) {
          failAndStop();
          return;
        }
        monitor = worker.scheduleWithFixedDelay(this::refreshTunnel, 0, 500, TimeUnit.MILLISECONDS);
      } catch (RuntimeException | LinkageError error) {
        failAndStop();
      }
    });
    return START_NOT_STICKY;
  }

  private void showForegroundNotification() {
    NotificationManager notifications = getSystemService(NotificationManager.class);
    notifications.createNotificationChannel(
        new NotificationChannel(
            CHANNEL, getString(R.string.vpn_channel), NotificationManager.IMPORTANCE_LOW));
    PendingIntent open =
        PendingIntent.getActivity(
            this,
            0,
            new Intent(this, MainActivity.class),
            PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    PendingIntent stop =
        PendingIntent.getService(
            this,
            1,
            new Intent(this, ZerouteVpnService.class).setAction(ACTION_STOP),
            PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    Notification notification =
        new Notification.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_launcher)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(getString(R.string.vpn_notification))
            .setContentIntent(open)
            .setOngoing(true)
            .addAction(
                new Notification.Action.Builder(null, getString(R.string.stop_engine), stop)
                    .build())
            .build();
    if (Build.VERSION.SDK_INT >= 34) {
      startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SYSTEM_EXEMPTED);
    } else {
      startForeground(1, notification);
    }
  }

  private void refreshTunnel() {
    if (closing) return;
    try {
      if (NativeBridge.readiness() == 0) {
        failAndStop();
        return;
      }
      String configuration = NativeBridge.vpnConfiguration();
      if (configuration.equals(appliedConfiguration)) return;
      if (configuration.isEmpty()) {
        closeTunnel();
        appliedConfiguration = "";
        return;
      }
      Builder builder =
          new Builder()
              .setSession(getString(R.string.app_name))
              .setBlocking(false)
              .allowFamily(OsConstants.AF_INET)
              .allowFamily(OsConstants.AF_INET6);
      if (Build.VERSION.SDK_INT >= 29) builder.setMetered(false);
      long revision = 0;
      for (String line : configuration.split("\n")) {
        String[] parts = line.split(" ", 2);
        if (parts.length != 2) throw new IllegalArgumentException("Invalid native configuration");
        switch (parts[0]) {
          case "revision":
            revision = Long.parseLong(parts[1]);
            break;
          case "mtu":
            builder.setMtu(Integer.parseInt(parts[1]));
            break;
          case "a":
            {
              String[] prefix = parts[1].split("/");
              builder.addAddress(prefix[0], Integer.parseInt(prefix[1]));
              break;
            }
          case "r":
            {
              String[] prefix = parts[1].split("/");
              builder.addRoute(prefix[0], Integer.parseInt(prefix[1]));
              break;
            }
          case "d":
            builder.addDnsServer(parts[1]);
            break;
          case "s":
            builder.addSearchDomain(parts[1]);
            break;
          default:
            throw new IllegalArgumentException("Unknown native configuration field");
        }
      }
      ParcelFileDescriptor replacement = builder.establish();
      if (replacement == null) throw new IllegalStateException("VPN permission was revoked");
      try {
        String error = NativeBridge.attachTunnel(replacement.getFd(), revision);
        if (!error.isEmpty()) throw new IllegalStateException("Native tunnel attachment failed");
        closeTunnel();
        tunnel = replacement;
        replacement = null;
        appliedConfiguration = configuration;
      } finally {
        if (replacement != null) replacement.close();
      }
    } catch (RuntimeException | IOException error) {
      failAndStop();
    }
  }

  private void closeTunnel() {
    if (tunnel != null) {
      try {
        tunnel.close();
      } catch (IOException ignored) {
        /* Already revoked. */
      }
      tunnel = null;
    }
  }

  private void failAndStop() {
    requestStop(true);
  }

  private synchronized void requestStop(boolean failure) {
    if (closing) return;
    closing = true;
    failed = failure;
    worker.execute(() -> {
      // The platform can keep a VPN service bound after stopSelf(). Release the
      // TUN and native transport first; onDestroy is only a cleanup fallback.
      cleanup();
      stopForeground(STOP_FOREGROUND_REMOVE);
      stopSelf();
    });
  }

  private void cleanup() {
    if (cleaned) return;
    cleaned = true;
    if (monitor != null) monitor.cancel(false);
    closeTunnel();
    try {
      NativeBridge.stopEngine();
      if (failed) NativeBridge.vpnFailed();
    } catch (RuntimeException | LinkageError ignored) {
      /* No state payloads are logged. */
    }
  }

  @Override
  public void onRevoke() {
    requestStop(false);
  }

  @Override
  public void onDestroy() {
    closing = true;
    worker.execute(this::cleanup);
    worker.shutdown();
    stopForeground(STOP_FOREGROUND_REMOVE);
    super.onDestroy();
  }
}
