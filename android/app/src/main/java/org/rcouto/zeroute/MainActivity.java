package org.rcouto.zeroute;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

public final class MainActivity extends Activity {
  @Override
  protected void onCreate(Bundle savedInstanceState) {
    super.onCreate(savedInstanceState);
    if (NativeBridge.apiVersion() != 1) {
      throw new IllegalStateException("Incompatible native component");
    }
    TextView status = new TextView(this);
    status.setText(R.string.bootstrap_status);
    status.setPadding(32, 32, 32, 32);
    setContentView(status);
  }
}
