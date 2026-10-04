package app.jizhang.wallet;

import android.os.Bundle;
import android.view.View;
import java.net.URL;
import javax.net.ssl.HttpsURLConnection;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import dev.gpui.mobile.GpuiInputActivity;

/** Android owns the surface and IME; Rust owns all wallet views and state. */
public final class WalletActivity extends GpuiInputActivity {
    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
    }

    // Called on Rust's background executor; uses Android's TLS and proxy settings.
    public String fetchExchangeRate() throws Exception {
        HttpsURLConnection connection = (HttpsURLConnection) new URL(
                "https://api.frankfurter.dev/v1/latest?base=USD&symbols=CNY").openConnection();
        connection.setConnectTimeout(15000);
        connection.setReadTimeout(15000);
        try {
            if (connection.getResponseCode() != 200) throw new java.io.IOException("Rate service unavailable");
            try (InputStream input = connection.getInputStream();
                 ByteArrayOutputStream output = new ByteArrayOutputStream()) {
                byte[] buffer = new byte[1024];
                int count;
                while ((count = input.read(buffer)) != -1) {
                    if (output.size() + count > 16384) throw new java.io.IOException("Rate response too large");
                    output.write(buffer, 0, count);
                }
                return output.toString("UTF-8");
            }
        } finally {
            connection.disconnect();
        }
    }

    public void setWalletTheme(boolean dark) {
        runOnUiThread(() -> {
            int background = dark ? 0xff17201c : 0xfff6f8f5;
            getWindow().setStatusBarColor(background);
            getWindow().setNavigationBarColor(background);
            int flags = getWindow().getDecorView().getSystemUiVisibility();
            int light = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
            getWindow().getDecorView().setSystemUiVisibility(dark ? flags & ~light : flags | light);
        });
    }
}
