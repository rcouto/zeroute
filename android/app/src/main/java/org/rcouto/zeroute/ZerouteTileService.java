package org.rcouto.zeroute;

import android.app.PendingIntent;
import android.content.Intent;
import android.os.Build;
import android.service.quicksettings.Tile;
import android.service.quicksettings.TileService;

/** On-demand Android panel adapter; no polling or additional background worker. */
public final class ZerouteTileService extends TileService {
  private final ConnectionController.Listener listener = this::render;

  @Override
  public void onStartListening() {
    super.onStartListening();
    try {
      render(ConnectionStatus.read());
      ConnectionController.subscribe(listener);
    } catch (RuntimeException | LinkageError error) {
      Tile tile = getQsTile();
      if (tile != null) {
        tile.setState(Tile.STATE_UNAVAILABLE);
        tile.updateTile();
      }
    }
  }

  @Override
  public void onStopListening() {
    ConnectionController.unsubscribe(listener);
    super.onStopListening();
  }

  @Override
  public void onDestroy() {
    ConnectionController.unsubscribe(listener);
    super.onDestroy();
  }

  private void render(ConnectionStatus status) {
    Tile tile = getQsTile();
    if (tile == null) return;
    tile.setState(status.active() ? Tile.STATE_ACTIVE : Tile.STATE_INACTIVE);
    if (Build.VERSION.SDK_INT >= 29) tile.setSubtitle(getString(status.title()));
    tile.setContentDescription(getString(R.string.app_name) + ": " + getString(status.title()));
    tile.updateTile();
  }

  @Override
  public void onClick() {
    super.onClick();
    if (isLocked()) unlockAndRun(this::openControl);
    else openControl();
  }

  private void openControl() {
    Intent intent =
        new Intent(this, VpnControlActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
    if (Build.VERSION.SDK_INT >= 34) {
      startActivityAndCollapse(
          PendingIntent.getActivity(
              this, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));
    } else {
      openLegacyControl(intent);
    }
  }

  // This overload is required below API 34; the caller gates it by runtime API.
  @android.annotation.SuppressLint("StartActivityAndCollapseDeprecated")
  @SuppressWarnings("deprecation")
  private void openLegacyControl(Intent intent) {
    startActivityAndCollapse(intent);
  }
}
