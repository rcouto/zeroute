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
  private long token;
  private volatile int latestStartId;
  private String notificationText = "";
  private boolean ownsUnderlay;
  private boolean cleaned;
  private boolean foreground;

  // Called synchronously by JNI before its transport worker starts.
  public boolean protectSocket(int descriptor) {
    return protect(descriptor);
  }

  @Override
  public int onStartCommand(Intent intent, int flags, int startId) {
    latestStartId = startId;
    if (intent == null) {
      requestStop(false);
      return START_NOT_STICKY;
    }
    if (ACTION_STOP.equals(intent.getAction())) {
      long requested = intent.getLongExtra("token", token);
      if (requested != 0 && requested == token) {
        NativeBridge.sessionEvent(token, 3);
        requestStop(false);
      } else if (token == 0 && NativeBridge.sessionAccepts(requested, 4)) {
        token = requested;
        requestStop(false);
      } else if (token == 0) {
        stopSelf(startId);
      }
      return START_NOT_STICKY;
    }
    long requested = intent.getLongExtra("token", 0);
    if (!ACTION_CONNECT.equals(intent.getAction()) || !NativeBridge.sessionAccepts(requested, 2)) {
      if (token == 0 && NativeBridge.sessionAccepts(requested, 4)) {
        token = requested;
        requestStop(false);
      } else if (token == 0) stopSelf(startId);
      return START_NOT_STICKY;
    }
    if (prepare(this) != null) {
      token = requested;
      NativeBridge.sessionFailure(token, 3);
      requestStop(true);
      return START_NOT_STICKY;
    }
    token = requested;
    closing = false;
    cleaned = false;
    if (!ownsUnderlay) {
      ConnectionController.acquireUnderlay(this);
      ownsUnderlay = true;
    }
    try {
      showForegroundNotification();
    } catch (RuntimeException error) {
      NativeBridge.sessionFailure(token, 6);
      requestStop(true);
      return START_NOT_STICKY;
    }
    final long attempt = token;
    worker.execute(() -> {
      if (closing || monitor != null || !NativeBridge.sessionAccepts(attempt, 2)) return;
      try {
        NativeBridge.stopEngine();
        String error = NativeBridge.startVpnEngine(getNoBackupFilesDir().getAbsolutePath(), this);
        if (!error.isEmpty()) {
          failAndStop();
          return;
        }
        if (!NativeBridge.sessionEvent(attempt, 2)) {
          requestStop(false);
          return;
        }
        ConnectionController.publish();
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
            new Intent(this, ZerouteVpnService.class)
                .setAction(ACTION_STOP)
                .putExtra("token", token),
            PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    Notification notification =
        new Notification.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_launcher)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(notificationStatus())
            .setContentIntent(open)
            .setOngoing(true)
            .addAction(
                new Notification.Action.Builder(null, getString(R.string.stop_engine), stop)
                    .build())
            .build();
    if (foreground) {
      notifications.notify(1, notification);
      return;
    }
    if (Build.VERSION.SDK_INT >= 34) {
      startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SYSTEM_EXEMPTED);
    } else {
      startForeground(1, notification);
    }
    foreground = true;
  }

  private String notificationStatus() {
    ConnectionStatus status = ConnectionStatus.read();
    return getString(status.title());
  }

  private void refreshTunnel() {
    if (closing) return;
    try {
      ConnectionController.publish();
      String text = notificationStatus();
      if (!text.equals(notificationText)) {
        notificationText = text;
        showForegroundNotification();
      }
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
    final long stoppingToken = token;
    if (failure) NativeBridge.sessionFailure(stoppingToken, 2);
    else NativeBridge.sessionEvent(stoppingToken, 3);
    ConnectionController.publish();
    if (foreground) {
      try {
        showForegroundNotification();
      } catch (RuntimeException ignored) {
        /* Teardown must not depend on notification delivery. */
      }
    }
    worker.execute(() -> {
      // The platform can keep a VPN service bound after stopSelf(). Release the
      // TUN and native transport first; onDestroy is only a cleanup fallback.
      if (cleanup()) {
        new android.os.Handler(android.os.Looper.getMainLooper())
            .post(() -> {
              stopForeground(STOP_FOREGROUND_REMOVE);
              foreground = false;
              stopSelf(latestStartId);
              NativeBridge.sessionEvent(stoppingToken, 4);
              ConnectionController.publish();
            });
      } else {
        NativeBridge.sessionFailure(stoppingToken, 5);
        ConnectionController.publish();
      }
    });
  }

  private boolean cleanup() {
    if (cleaned) return true;
    if (monitor != null) {
      monitor.cancel(false);
      monitor = null;
    }
    closeTunnel();
    appliedConfiguration = "";
    try {
      String error = NativeBridge.stopEngine();
      if (!error.isEmpty()) return false;
      cleaned = true;
      return true;
    } catch (RuntimeException | LinkageError ignored) {
      return false;
    }
  }

  @Override
  public void onRevoke() {
    NativeBridge.sessionFailure(token, 3);
    requestStop(true);
  }

  @Override
  public void onDestroy() {
    closing = true;
    if (ownsUnderlay) {
      ConnectionController.releaseUnderlay();
      ownsUnderlay = false;
    }
    final long destroyedToken = token;
    worker.execute(() -> {
      if (NativeBridge.sessionAccepts(destroyedToken, 3)
          || NativeBridge.sessionAccepts(destroyedToken, 2))
        NativeBridge.sessionFailure(destroyedToken, 4);
      if (NativeBridge.sessionAccepts(destroyedToken, 4)) {
        if (cleanup()) NativeBridge.sessionEvent(destroyedToken, 4);
        else NativeBridge.sessionFailure(destroyedToken, 5);
        ConnectionController.publish();
      }
    });
    worker.shutdown();
    stopForeground(STOP_FOREGROUND_REMOVE);
    super.onDestroy();
  }
}
