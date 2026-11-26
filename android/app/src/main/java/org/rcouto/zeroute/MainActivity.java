package org.rcouto.zeroute;

import android.app.Activity;
import android.app.AlertDialog;
import android.app.StatusBarManager;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.ComponentName;
import android.content.Intent;
import android.graphics.drawable.Icon;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;
import android.text.TextUtils;
import android.view.View;
import android.view.WindowInsets;
import android.webkit.WebResourceRequest;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.TextView;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Adaptive framework presentation; Rust owns connection and lifecycle semantics. */
public final class MainActivity extends Activity {
  private final ExecutorService worker = Executors.newSingleThreadExecutor();
  private final ConnectionController.Listener listener = this::render;
  private AlertDialog noticesDialog;
  private boolean loaded;
  private boolean configured;
  private boolean saving;
  private boolean observing;
  private EditText networkId;
  private ConnectionStatus status;

  @Override
  protected void onCreate(Bundle saved) {
    boolean contrast = getPreferences(MODE_PRIVATE).getBoolean("highContrast", false);
    if (contrast) setTheme(R.style.ZerouteHighContrast);
    super.onCreate(saved);
    setContentView(R.layout.activity_main);
    if (Build.VERSION.SDK_INT >= 28) {
      findViewById(R.id.app_title).setAccessibilityHeading(true);
      findViewById(R.id.connection_status).setAccessibilityHeading(true);
    }
    View root = findViewById(R.id.root);
    root.setOnApplyWindowInsetsListener((view, insets) -> {
      if (Build.VERSION.SDK_INT >= 30) {
        android.graphics.Insets bars =
            insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.ime());
        view.setPadding(bars.left, bars.top, bars.right, bars.bottom);
      } else applyLegacyInsets(view, insets);
      return insets;
    });
    root.requestApplyInsets();
    View content = findViewById(R.id.content);
    content.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> {
      int base = (int) (24 * getResources().getDisplayMetrics().density);
      int padding =
          Math.max(base, (r - l - (int) (640 * getResources().getDisplayMetrics().density)) / 2);
      if (v.getPaddingLeft() != padding) v.setPadding(padding, base, padding, base);
    });
    networkId = findViewById(R.id.network_id);
    if (saved != null && saved.getBoolean("detailsExpanded")) showDetails(true);
    findViewById(R.id.connection_action).setEnabled(false);
    findViewById(R.id.save_network).setOnClickListener(v -> saveNetwork());
    findViewById(R.id.connection_action)
        .setOnClickListener(v -> {
          if (status == null) return;
          if (status.active()) ConnectionController.stop(this);
          else if (!configured) {
            showDetails(true);
            networkId.requestFocus();
          } else ConnectionController.connect(this);
        });
    findViewById(R.id.toggle_details)
        .setOnClickListener(
            v -> showDetails(findViewById(R.id.details).getVisibility() != View.VISIBLE));
    View.OnClickListener copyNode =
        v -> {
          if (status != null && !status.node.isEmpty()) {
            getSystemService(ClipboardManager.class)
                .setPrimaryClip(ClipData.newPlainText(getString(R.string.node_label), status.node));
            setText(R.id.message, getString(R.string.node_copied));
          }
        };
    findViewById(R.id.copy_node).setOnClickListener(copyNode);
    findViewById(R.id.copy_admission).setOnClickListener(copyNode);
    findViewById(R.id.portal_action)
        .setOnClickListener(v -> {
          try {
            startActivity(new Intent(Settings.ACTION_WIFI_SETTINGS));
          } catch (RuntimeException error) {
            setText(R.id.message, getString(R.string.open_settings_manually));
          }
        });
    CheckBox contrastBox = findViewById(R.id.high_contrast);
    contrastBox.setChecked(contrast);
    contrastBox.setOnCheckedChangeListener((button, checked) -> {
      getPreferences(MODE_PRIVATE).edit().putBoolean("highContrast", checked).apply();
      recreate();
    });
    findViewById(R.id.add_tile).setOnClickListener(v -> addTile());
    findViewById(R.id.core_license).setOnClickListener(v -> showLicense());
    try {
      status = ConnectionStatus.read();
      setText(R.id.core_version, getString(R.string.core_version, NativeBridge.engineVersion()));
      worker.execute(() -> {
        String stored = "";
        boolean success = false;
        try {
          stored = NativeBridge.loadNetworkId(getNoBackupFilesDir().getAbsolutePath());
          success = true;
        } catch (RuntimeException ignored) {
          /* No private state is logged. */
        }
        final String value = stored;
        final boolean ok = success;
        runOnUiThread(() -> {
          if (isDestroyed()) return;
          loaded = true;
          configured = !value.isEmpty();
          if (saved == null) networkId.setText(value);
          if (!configured) showDetails(true);
          render(ConnectionStatus.read());
          if (!ok) setText(R.id.message, getString(R.string.settings_unavailable));
        });
      });
      render(status);
    } catch (RuntimeException | LinkageError error) {
      setText(R.id.connection_status, getString(R.string.native_unavailable));
    }
  }

  @SuppressWarnings("deprecation")
  private void applyLegacyInsets(View view, WindowInsets insets) {
    view.setPadding(
        insets.getSystemWindowInsetLeft(),
        insets.getSystemWindowInsetTop(),
        insets.getSystemWindowInsetRight(),
        insets.getSystemWindowInsetBottom());
  }

  private void showDetails(boolean show) {
    findViewById(R.id.details).setVisibility(show ? View.VISIBLE : View.GONE);
    ((Button) findViewById(R.id.toggle_details))
        .setText(show ? R.string.details_hide : R.string.details_show);
  }

  private void saveNetwork() {
    if (status == null || status.active() || saving) return;
    String input = networkId.getText().toString();
    saving = true;
    render(status);
    worker.execute(() -> {
      String failure;
      try {
        failure = NativeBridge.saveNetworkId(getNoBackupFilesDir().getAbsolutePath(), input);
      } catch (RuntimeException error) {
        failure = getString(R.string.settings_save_failed);
      }
      final String result = failure;
      runOnUiThread(() -> {
        if (isDestroyed()) return;
        saving = false;
        if (result.isEmpty()) {
          configured = true;
          networkId.setError(null);
          setText(R.id.message, getString(R.string.settings_saved));
        } else networkId.setError(result);
        ConnectionController.publish();
      });
    });
  }

  private void render(ConnectionStatus current) {
    if (isDestroyed()) return;
    status = current;
    boolean setup = loaded && !configured && !current.active();
    setText(R.id.connection_status, getString(setup ? R.string.state_setup : current.title()));
    setText(
        R.id.status_description,
        getString(setup ? R.string.description_setup : current.description()));
    Button action = findViewById(R.id.connection_action);
    int actionTitle =
        current.phase == 4
            ? R.string.state_stopping
            : current.active()
                ? (current.phase == 1 || current.phase == 2
                    ? R.string.cancel_connect
                    : R.string.stop_engine)
                : setup
                    ? R.string.setup_action
                    : current.phase == 5 ? R.string.retry : R.string.start_engine;
    setText(R.id.connection_action, getString(actionTitle));
    action.setEnabled(loaded && !saving && current.phase != 4);
    findViewById(R.id.save_network).setEnabled(loaded && !saving && !current.active());
    networkId.setEnabled(loaded && !saving && !current.active());
    setText(
        R.id.tunnel_status,
        getString(
            R.string.stage_status,
            getString(R.string.stage_tunnel),
            getString(current.stage >= 4 ? R.string.stage_ready : R.string.stage_waiting)));
    setText(
        R.id.dns_status,
        getString(
            R.string.stage_status,
            getString(R.string.stage_dns),
            getString(
                current.stage >= 3 && !current.dnsConfigured
                    ? R.string.dns_not_configured
                    : current.state == 6 ? R.string.dns_responding : R.string.dns_no_evidence)));
    setText(
        R.id.control_plane_status,
        getString(
            R.string.control_plane_status,
            getString(current.roots ? R.string.stage_ready : R.string.stage_waiting)));
    setText(
        R.id.node_address,
        getString(
            R.string.node_address,
            current.node.isEmpty() ? getString(R.string.unknown) : current.node));
    findViewById(R.id.copy_node).setEnabled(!current.node.isEmpty());
    findViewById(R.id.admission).setVisibility(current.state == 8 ? View.VISIBLE : View.GONE);
    setText(
        R.id.admission_node,
        getString(
            R.string.node_address,
            current.node.isEmpty() ? getString(R.string.unknown) : current.node));
    findViewById(R.id.copy_admission).setEnabled(!current.node.isEmpty());
    findViewById(R.id.portal_action)
        .setVisibility((current.network & 8) != 0 ? View.VISIBLE : View.GONE);
    StringBuilder transport = new StringBuilder();
    if ((current.network & 16) != 0) transport.append(getString(R.string.wifi));
    if ((current.network & 32) != 0) {
      if (transport.length() > 0) transport.append(" / ");
      transport.append(getString(R.string.mobile));
    }
    if ((current.network & 64) != 0) {
      if (transport.length() > 0) transport.append(" / ");
      transport.append(getString(R.string.ethernet));
    }
    if (transport.length() == 0)
      transport.append(
          getString(
              (current.network & 1) == 0
                  ? R.string.unknown
                  : (current.network & 2) != 0 ? R.string.other_network : R.string.no_network));
    setText(R.id.underlay_status, getString(R.string.available_networks, transport));
    String notice =
        (current.network & 8) != 0
            ? getString(R.string.portal_notice)
            : (current.network & 1) != 0 && (current.network & 2) != 0 && (current.network & 4) == 0
                ? getString(R.string.validation_unknown)
                : "";
    setText(R.id.network_notice, notice);
    setText(R.id.engine_error, current.error);
    setText(
        R.id.scope_note,
        getString(current.phase == 0 ? R.string.scope_disconnected : R.string.scope_private));
  }

  private void setText(int id, String value) {
    TextView view = findViewById(id);
    if (!value.contentEquals(view.getText())) view.setText(value);
  }

  private void addTile() {
    if (Build.VERSION.SDK_INT >= 33)
      getSystemService(StatusBarManager.class)
          .requestAddTileService(
              new ComponentName(this, ZerouteTileService.class),
              getString(R.string.app_name),
              Icon.createWithResource(this, R.drawable.ic_vpn_tile),
              getMainExecutor(),
              result -> {});
    else setText(R.id.message, getString(R.string.tile_help));
  }

  private void showLicense() {
    String index = "file:///android_asset/notices/index.html";
    WebView notices = new WebView(this);
    notices.getSettings().setAllowFileAccess(false);
    notices.getSettings().setAllowContentAccess(false);
    notices.getSettings().setBlockNetworkLoads(true);
    notices.setWebViewClient(
        new WebViewClient() {
          @Override
          public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
            Uri uri = request.getUrl();
            if ("file".equals(uri.getScheme()) && TextUtils.isEmpty(uri.getEncodedAuthority())) {
              String path = uri.getPath();
              return path == null
                  || !path.startsWith("/android_asset/notices/")
                  || path.contains("..");
            }
            if ("https".equals(uri.getScheme())
                && request.hasGesture()
                && request.isForMainFrame()) {
              try {
                startActivity(new Intent(Intent.ACTION_VIEW, uri));
              } catch (ActivityNotFoundException ignored) {
                setText(R.id.message, getString(R.string.source_link_unavailable));
              }
            }
            return true;
          }
        });
    noticesDialog =
        new AlertDialog.Builder(this)
            .setTitle(R.string.core_license)
            .setView(notices)
            .setPositiveButton(android.R.string.ok, null)
            .setNeutralButton(R.string.notices_index, null)
            .create();
    noticesDialog.setOnDismissListener(dialog -> {
      if (notices.getParent() instanceof android.view.ViewGroup parent) {
        parent.removeView(notices);
      }
      notices.destroy();
      noticesDialog = null;
    });
    noticesDialog.show();
    noticesDialog
        .getButton(AlertDialog.BUTTON_NEUTRAL)
        .setOnClickListener(v -> notices.loadUrl(index));
    notices.loadUrl(index);
  }

  @Override
  protected void onSaveInstanceState(Bundle out) {
    out.putBoolean("detailsExpanded", findViewById(R.id.details).getVisibility() == View.VISIBLE);
    super.onSaveInstanceState(out);
  }

  @Override
  protected void onResume() {
    super.onResume();
    if (status != null) {
      ConnectionController.acquireUnderlay(this);
      observing = true;
    }
    ConnectionController.subscribe(listener);
  }

  @Override
  protected void onPause() {
    ConnectionController.unsubscribe(listener);
    if (observing) {
      ConnectionController.releaseUnderlay();
      observing = false;
    }
    super.onPause();
  }

  @Override
  protected void onDestroy() {
    ConnectionController.unsubscribe(listener);
    if (noticesDialog != null) noticesDialog.dismiss();
    worker.shutdown();
    super.onDestroy();
  }
}
