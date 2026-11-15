package org.rcouto.zeroute;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.VpnService;
import android.os.Build;
import android.os.Bundle;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** One private consent entry point for app and tile; native tokens reject stale results. */
public final class VpnControlActivity extends Activity {
  private final ExecutorService worker = Executors.newSingleThreadExecutor();
  private long token;
  private boolean awaitingResult;
  private boolean handedOff;

  @Override
  protected void onCreate(Bundle saved) {
    super.onCreate(saved);
    try {
      if (saved != null) {
        token = saved.getLong("token");
        awaitingResult = saved.getBoolean("awaitingResult");
        handedOff = saved.getBoolean("handedOff");
        if (!NativeBridge.sessionAccepts(token, 1)) {
          finish();
          return;
        }
        if (awaitingResult) return;
      } else {
        if (getIntent().getBooleanExtra("stopOnly", false)) {
          ConnectionController.stop(this);
          finish();
          return;
        }
        if (!getIntent().getBooleanExtra("connectOnly", false)
            && ConnectionStatus.read().active()) {
          ConnectionController.stop(this);
          finish();
          return;
        }
        token = NativeBridge.beginSession();
        if (token == 0) {
          finish();
          return;
        }
      }
      ConnectionController.publish();
      worker.execute(() -> {
        boolean configured;
        try {
          configured =
              !NativeBridge.loadNetworkId(getNoBackupFilesDir().getAbsolutePath()).isEmpty();
        } catch (RuntimeException error) {
          configured = false;
        }
        final boolean ready = configured;
        runOnUiThread(() -> {
          if (isDestroyed() || isFinishing()) return;
          if (!NativeBridge.sessionAccepts(token, 1)) {
            finish();
            return;
          }
          if (ready) prepareVpn();
          else {
            NativeBridge.sessionEvent(token, 5);
            startActivity(new Intent(this, MainActivity.class));
            finish();
          }
        });
      });
    } catch (RuntimeException | LinkageError error) {
      finish();
    }
  }

  @SuppressWarnings("deprecation")
  private void prepareVpn() {
    if (!NativeBridge.sessionAccepts(token, 1)) {
      finish();
      return;
    }
    Intent consent = VpnService.prepare(this);
    if (consent != null) {
      awaitingResult = true;
      startActivityForResult(consent, 100);
    } else notificationPermission();
  }

  private void notificationPermission() {
    if (!NativeBridge.sessionAccepts(token, 1)) {
      finish();
      return;
    }
    if (Build.VERSION.SDK_INT >= 33
        && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
            != PackageManager.PERMISSION_GRANTED) {
      awaitingResult = true;
      requestPermissions(new String[] {Manifest.permission.POST_NOTIFICATIONS}, 101);
    } else startVpn();
  }

  private void startVpn() {
    awaitingResult = false;
    if (NativeBridge.sessionEvent(token, 1)) {
      try {
        startForegroundService(
            new Intent(this, ZerouteVpnService.class)
                .setAction(ZerouteVpnService.ACTION_CONNECT)
                .putExtra("token", token));
        handedOff = true;
      } catch (RuntimeException error) {
        NativeBridge.sessionFailure(token, 6);
        NativeBridge.sessionEvent(token, 4);
      }
    }
    ConnectionController.publish();
    finish();
  }

  @Override
  @SuppressWarnings("deprecation")
  protected void onActivityResult(int code, int result, Intent data) {
    super.onActivityResult(code, result, data);
    awaitingResult = false;
    if (code == 100 && result == RESULT_OK) notificationPermission();
    else {
      NativeBridge.sessionEvent(token, 5);
      ConnectionController.publish();
      finish();
    }
  }

  @Override
  public void onRequestPermissionsResult(int code, String[] permissions, int[] results) {
    super.onRequestPermissionsResult(code, permissions, results);
    if (code == 101) startVpn();
  }

  @Override
  protected void onSaveInstanceState(Bundle out) {
    out.putLong("token", token);
    out.putBoolean("awaitingResult", awaitingResult);
    out.putBoolean("handedOff", handedOff);
    super.onSaveInstanceState(out);
  }

  @Override
  protected void onDestroy() {
    if (isFinishing() && !handedOff) {
      try {
        NativeBridge.sessionEvent(token, 5);
        ConnectionController.publish();
      } catch (RuntimeException | LinkageError ignored) {
        /* Native load failure is handled by the main screen. */
      }
    }
    worker.shutdown();
    super.onDestroy();
  }
}
