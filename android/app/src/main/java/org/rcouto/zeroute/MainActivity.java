package org.rcouto.zeroute;

import android.Manifest;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.VpnService;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.WindowInsets;
import android.widget.Button;
import android.widget.EditText;
import android.widget.TextView;
import java.util.Scanner;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class MainActivity extends Activity {
  private final Handler handler = new Handler(Looper.getMainLooper());
  private final ExecutorService settingsWorker = Executors.newSingleThreadExecutor();
  private EditText networkId;
  private Button save;
  private boolean nativeReady;
  private boolean settingsReady;
  private boolean operationPending;
  private boolean notificationsAsked;
  private String lastEngineError = "";
  private Button startEngine;
  private Button stopEngine;

  private final Runnable refresh =
      new Runnable() {
        @Override
        public void run() {
          if (!nativeReady) return;
          renderReadiness();
          if (nativeReady) handler.postDelayed(this, 1000);
        }
      };

  @Override
  protected void onCreate(Bundle savedInstanceState) {
    super.onCreate(savedInstanceState);
    setContentView(R.layout.activity_main);
    findViewById(R.id.root)
        .setOnApplyWindowInsetsListener((view, insets) -> {
          if (Build.VERSION.SDK_INT >= 30) {
            android.graphics.Insets bars =
                insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.ime());
            view.setPadding(bars.left, bars.top, bars.right, bars.bottom);
          } else {
            applyLegacyInsets(view, insets);
          }
          return insets;
        });
    findViewById(R.id.root).requestApplyInsets();
    findViewById(R.id.core_license).setOnClickListener(view -> showCoreLicense());
    networkId = findViewById(R.id.network_id);
    save = findViewById(R.id.save_network);
    startEngine = findViewById(R.id.start_engine);
    stopEngine = findViewById(R.id.stop_engine);
    startEngine.setEnabled(false);
    stopEngine.setEnabled(false);
    networkId.setEnabled(false);
    save.setEnabled(false);
    try {
      if (NativeBridge.apiVersion() != NativeBridge.API_VERSION) {
        throw new IllegalStateException("Native ABI mismatch");
      }
      nativeReady = true;
      ((TextView) findViewById(R.id.core_version))
          .setText(getString(R.string.core_version, NativeBridge.engineVersion()));
    } catch (RuntimeException | LinkageError error) {
      showMessage(R.string.native_unavailable);
      return;
    }
    settingsWorker.execute(() -> {
      String stored = "";
      boolean loaded = false;
      try {
        stored = NativeBridge.loadNetworkId(getNoBackupFilesDir().getAbsolutePath());
        loaded = true;
      } catch (RuntimeException error) {
        // Show a generic error; never log native state or identity material.
      }
      final String savedId = stored;
      final boolean settingsLoaded = loaded;
      handler.post(() -> {
        if (isDestroyed()) return;
        if (savedInstanceState == null) networkId.setText(savedId);
        settingsReady = true;
        renderReadiness();
        if (!settingsLoaded) showMessage(R.string.settings_unavailable);
      });
    });
    save.setOnClickListener(view -> saveNetwork());
    startEngine.setOnClickListener(view -> requestVpn());
    stopEngine.setOnClickListener(view ->
        startService(
            new Intent(this, ZerouteVpnService.class).setAction(ZerouteVpnService.ACTION_STOP)));
  }

  @SuppressWarnings("deprecation")
  private void applyLegacyInsets(android.view.View view, WindowInsets insets) {
    view.setPadding(
        insets.getSystemWindowInsetLeft(),
        insets.getSystemWindowInsetTop(),
        insets.getSystemWindowInsetRight(),
        insets.getSystemWindowInsetBottom());
  }

  private void showCoreLicense() {
    settingsWorker.execute(() -> {
      String text;
      try (Scanner scanner =
          new Scanner(getResources().openRawResource(R.raw.zerotier_core_license), "UTF-8")) {
        text = scanner.useDelimiter("\\A").next();
      }
      final String license = text;
      handler.post(() -> {
        if (!isDestroyed() && !isFinishing()) {
          new AlertDialog.Builder(this)
              .setTitle(R.string.core_license)
              .setMessage(license)
              .setPositiveButton(android.R.string.ok, null)
              .show();
        }
      });
    });
  }

  private void saveNetwork() {
    final String input = networkId.getText().toString();
    operationPending = true;
    save.setEnabled(false);
    settingsWorker.execute(() -> {
      String failure;
      try {
        failure = NativeBridge.saveNetworkId(getNoBackupFilesDir().getAbsolutePath(), input);
      } catch (RuntimeException error) {
        failure = getString(R.string.settings_save_failed);
      }
      final String message = failure;
      handler.post(() -> {
        if (isDestroyed()) return;
        operationPending = false;
        renderReadiness();
        if (message.isEmpty()) {
          networkId.setError(null);
          showMessage(R.string.settings_saved);
        } else {
          networkId.setError(message);
          showMessage(R.string.settings_save_failed);
        }
      });
    });
  }

  @SuppressWarnings("deprecation")
  private void requestVpn() {
    Intent consent = VpnService.prepare(this);
    if (consent != null) startActivityForResult(consent, 100);
    else connectVpn();
  }

  private void connectVpn() {
    if (Build.VERSION.SDK_INT >= 33
        && !notificationsAsked
        && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
            != PackageManager.PERMISSION_GRANTED) {
      notificationsAsked = true;
      requestPermissions(new String[] {Manifest.permission.POST_NOTIFICATIONS}, 101);
      return;
    }
    startVpnService();
  }

  @Override
  public void onRequestPermissionsResult(
      int requestCode, String[] permissions, int[] grantResults) {
    super.onRequestPermissionsResult(requestCode, permissions, grantResults);
    if (requestCode == 101) startVpnService();
  }

  private void startVpnService() {
    startForegroundService(
        new Intent(this, ZerouteVpnService.class).setAction(ZerouteVpnService.ACTION_CONNECT));
  }

  @Override
  @SuppressWarnings("deprecation")
  protected void onActivityResult(int requestCode, int resultCode, Intent data) {
    super.onActivityResult(requestCode, resultCode, data);
    if (requestCode == 100 && resultCode == RESULT_OK) connectVpn();
  }

  private void renderReadiness() {
    if (!nativeReady) return;
    try {
      long state = NativeBridge.readiness();
      if (state < 0 || state > 5) throw new IllegalStateException("Unknown readiness state");
      boolean active = state != 0;
      boolean available = (NativeBridge.capabilities() & NativeBridge.CAPABILITY_ENGINE) != 0;
      startEngine.setEnabled(available && settingsReady && !active && !operationPending);
      stopEngine.setEnabled(active && !operationPending);
      save.setEnabled(settingsReady && !active && !operationPending);
      networkId.setEnabled(settingsReady && !active && !operationPending);
      String address = NativeBridge.nodeAddress();
      ((TextView) findViewById(R.id.node_address))
          .setText(
              getString(
                  R.string.node_address,
                  address.isEmpty() ? getString(R.string.unknown) : address));
      ((TextView) findViewById(R.id.control_plane_status))
          .setText(
              getString(
                  R.string.control_plane_status,
                  getString(
                      NativeBridge.controlPlaneOnline()
                          ? R.string.stage_ready
                          : R.string.stage_waiting)));
      ((TextView) findViewById(R.id.traffic_status)).setText(NativeBridge.trafficDiagnostics());
      String failure = NativeBridge.engineError();
      if (!failure.equals(lastEngineError)) {
        ((TextView) findViewById(R.id.message)).setText(failure);
        lastEngineError = failure;
      }
      int[] titles = {
        R.string.state_stopped,
        R.string.state_engine,
        R.string.state_configuration,
        R.string.state_tunnel,
        R.string.state_dns,
        R.string.state_ready
      };
      ((TextView) findViewById(R.id.connection_status)).setText(titles[(int) state]);
      renderStage(R.id.engine_status, R.string.stage_engine, state >= 2);
      renderStage(R.id.configuration_status, R.string.stage_configuration, state >= 3);
      renderStage(R.id.tunnel_status, R.string.stage_tunnel, state >= 4);
      renderStage(R.id.dns_status, R.string.stage_dns, state == 5);
    } catch (RuntimeException error) {
      nativeReady = false;
      showMessage(R.string.native_unavailable);
    }
  }

  private void renderStage(int viewId, int title, boolean ready) {
    ((TextView) findViewById(viewId))
        .setText(
            getString(
                R.string.stage_status,
                getString(title),
                getString(ready ? R.string.stage_ready : R.string.stage_waiting)));
  }

  private void showMessage(int message) {
    ((TextView) findViewById(R.id.message)).setText(message);
  }

  @Override
  protected void onResume() {
    super.onResume();
    handler.post(refresh);
  }

  @Override
  protected void onPause() {
    handler.removeCallbacks(refresh);
    super.onPause();
  }

  @Override
  protected void onDestroy() {
    handler.removeCallbacksAndMessages(null);
    settingsWorker.shutdown();
    super.onDestroy();
  }
}
