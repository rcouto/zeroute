package org.rcouto.zeroute;

import android.app.PendingIntent;
import android.content.Intent;
import android.os.Build;
import android.service.quicksettings.Tile;
import android.service.quicksettings.TileService;

/** On-demand Android panel adapter; no polling or additional background worker. */
public final class ZerouteTileService extends TileService {
  @Override
  public void onStartListening() {
    super.onStartListening();
    Tile tile = getQsTile();
    if (tile == null) return;
    try {
      long state = NativeBridge.readiness();
      tile.setState(state == 0 ? Tile.STATE_INACTIVE : Tile.STATE_ACTIVE);
      if (Build.VERSION.SDK_INT >= 29) {
        tile.setSubtitle(
            getString(
                state == 0
                    ? R.string.state_stopped
                    : state == 5 ? R.string.state_ready : R.string.tile_connecting));
      }
    } catch (RuntimeException | LinkageError error) {
      tile.setState(Tile.STATE_UNAVAILABLE);
    }
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
