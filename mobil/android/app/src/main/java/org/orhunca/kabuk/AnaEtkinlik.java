package org.orhunca.kabuk;

import android.Manifest;
import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Insets;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.view.View;
import android.view.WindowInsets;
import android.webkit.JavascriptInterface;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

/** Orhunca arayüz programını tam ekran gösterir; telefon özelliklerini sayfaya açar. */
public class AnaEtkinlik extends Activity {
    private WebView sayfa;

    @Override
    protected void onCreate(Bundle durum) {
        super.onCreate(durum);
        sayfa = new WebView(this);
        WebSettings a = sayfa.getSettings();
        a.setJavaScriptEnabled(true);
        a.setDomStorageEnabled(true);
        a.setAllowFileAccess(true);
        a.setMediaPlaybackRequiresUserGesture(false);
        sayfa.addJavascriptInterface(new Kopru(), "OrhuncaMobil");
        sayfa.setWebChromeClient(new WebChromeClient());
        sayfa.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView w, WebResourceRequest istek) {
                // Dış bağlantılar tarayıcıda açılır
                Uri u = istek.getUrl();
                if ("http".equals(u.getScheme()) || "https".equals(u.getScheme())) {
                    startActivity(new Intent(Intent.ACTION_VIEW, u));
                    return true;
                }
                return false;
            }
        });
        // Android 15'te uygulama kenardan kenara çizilir: sistem çubukları kadar boşluk bırakılır.
        if (Build.VERSION.SDK_INT >= 30) {
            sayfa.setOnApplyWindowInsetsListener((v, ic) -> {
                Insets i = ic.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.ime());
                v.setPadding(i.left, i.top, i.right, i.bottom);
                return WindowInsets.CONSUMED;
            });
        }
        setContentView(sayfa);
        if (durum != null) sayfa.restoreState(durum);
        else sayfa.loadUrl("file:///android_asset/uygulama.html");
    }

    @Override
    protected void onSaveInstanceState(Bundle durum) {
        super.onSaveInstanceState(durum);
        sayfa.saveState(durum);
    }

    @Override
    public void onBackPressed() {
        if (sayfa.canGoBack()) sayfa.goBack();
        else super.onBackPressed();
    }

    /** Sayfadaki `window.OrhuncaMobil` nesnesi (Orhunca: titret, paylaş, bildirim_gönder). */
    private class Kopru {
        @JavascriptInterface
        public void titret(int ms) {
            Vibrator v = (Vibrator) getSystemService(Context.VIBRATOR_SERVICE);
            if (v == null) return;
            int sure = Math.max(1, Math.min(ms, 5000));
            if (Build.VERSION.SDK_INT >= 26) v.vibrate(VibrationEffect.createOneShot(sure, VibrationEffect.DEFAULT_AMPLITUDE));
            else v.vibrate(sure);
        }

        @JavascriptInterface
        public void paylas(String metin) {
            Intent i = new Intent(Intent.ACTION_SEND);
            i.setType("text/plain");
            i.putExtra(Intent.EXTRA_TEXT, metin);
            startActivity(Intent.createChooser(i, null));
        }

        @JavascriptInterface
        public void bildirim(String baslik, String metin) {
            runOnUiThread(() -> bildirimGoster(baslik, metin));
        }
    }

    private static int bildirimSayaci = 1;

    private void bildirimGoster(String baslik, String metin) {
        if (Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] {Manifest.permission.POST_NOTIFICATIONS}, 1);
            return;
        }
        NotificationManager y = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
        if (y == null) return;
        Notification.Builder b;
        if (Build.VERSION.SDK_INT >= 26) {
            y.createNotificationChannel(new NotificationChannel("orhunca", "Bildirimler", NotificationManager.IMPORTANCE_DEFAULT));
            b = new Notification.Builder(this, "orhunca");
        } else {
            b = new Notification.Builder(this);
        }
        b.setSmallIcon(getApplicationInfo().icon).setContentTitle(baslik).setContentText(metin).setAutoCancel(true);
        y.notify(bildirimSayaci++, b.build());
    }
}
