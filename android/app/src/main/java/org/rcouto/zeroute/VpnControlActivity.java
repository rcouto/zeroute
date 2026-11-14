package org.rcouto.zeroute;

import android.app.Activity;
import android.content.Intent;
import android.net.VpnService;
import android.os.Bundle;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Private foreground entry point for the Quick Settings toggle and VPN consent. */
public final class VpnControlActivity extends Activity {
  private boolean waitingForConsent;
  private final ExecutorService worker = Executors.newSingleThreadExecutor();

  @Override
  protected void onCreate(Bundle state) {
    super.onCreate(state);
    waitingForConsent = state != null && state.getBoolean("waitingForConsent");
    if (waitingForConsent) return; // Android restores the outstanding consent result.
    try {
      if (NativeBridge.readiness() != 0) {
        startService(
            new Intent(this, ZerouteVpnService.class).setAction(ZerouteVpnService.ACTION_STOP));
        finish();
        return;
      }
    } catch (RuntimeException | LinkageError error) {
      openSetup();
      return;
    }
    worker.execute(() -> {
      boolean configured;
      try {
        configured = !NativeBridge.loadNetworkId(getNoBackupFilesDir().getAbsolutePath()).isEmpty();
      } catch (RuntimeException error) {
        configured = false;
      }
      final boolean ready = configured;
      runOnUiThread(() -> {
        if (isDestroyed() || isFinishing()) return;
        if (ready) requestVpn();
        else openSetup();
      });
    });
  }

  @SuppressWarnings("deprecation")
  private void requestVpn() {
    Intent consent = VpnService.prepare(this);
    if (consent != null) {
      waitingForConsent = true;
      startActivityForResult(consent, 100);
    } else connect();
  }

  private void connect() {
    startForegroundService(
        new Intent(this, ZerouteVpnService.class).setAction(ZerouteVpnService.ACTION_CONNECT));
    finish();
  }

  private void openSetup() {
    startActivity(new Intent(this, MainActivity.class));
    finish();
  }

  @Override
  @SuppressWarnings("deprecation")
  protected void onActivityResult(int requestCode, int resultCode, Intent data) {
    super.onActivityResult(requestCode, resultCode, data);
    if (requestCode == 100 && resultCode == RESULT_OK) connect();
    else finish();
  }

  @Override
  protected void onSaveInstanceState(Bundle state) {
    state.putBoolean("waitingForConsent", waitingForConsent);
    super.onSaveInstanceState(state);
  }

  @Override
  protected void onDestroy() {
    worker.shutdown();
    super.onDestroy();
  }
}
