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
import android.os.Handler;
import android.os.Looper;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.util.Log;
import android.view.View;
import android.webkit.ConsoleMessage;
import android.webkit.GeolocationPermissions;
import android.webkit.PermissionRequest;
import android.webkit.ValueCallback;
import android.view.WindowInsets;
import android.webkit.JavascriptInterface;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.FrameLayout;

/** Orhunca arayüz programını tam ekran gösterir; telefon özelliklerini sayfaya açar. */
public class AnaEtkinlik extends Activity {
    private WebView sayfa;
    /** İzin istenirken gönderilmek istenen bildirim (izin verilince gösterilir). */
    private String[] bekleyenBildirim;
    /** Kamera/konum izni beklenirken sayfanın isteği (izin gelince yanıtlanır). */
    private PermissionRequest bekleyenKamera;
    private GeolocationPermissions.Callback bekleyenKonum;
    private String bekleyenKonumKaynagi;
    /** Dosya seçme penceresinin sonucu sayfaya bununla döner. */
    private ValueCallback<Uri[]> dosyaSecimi;

    @Override
    protected void onCreate(Bundle durum) {
        super.onCreate(durum);
        sayfa = new WebView(this);
        WebSettings a = sayfa.getSettings();
        a.setJavaScriptEnabled(true);
        a.setDomStorageEnabled(true);
        a.setAllowFileAccess(true);
        a.setMediaPlaybackRequiresUserGesture(false);
        a.setGeolocationEnabled(true);
        sayfa.addJavascriptInterface(new Kopru(), "OrhuncaMobil");
        // Sayfadaki hatalar ve console.log iletileri logcat'e (etiket: Orhunca)
        sayfa.setWebChromeClient(new WebChromeClient() {
            @Override
            public boolean onConsoleMessage(ConsoleMessage m) {
                Log.i("Orhunca", m.message() + " (" + m.sourceId() + ":" + m.lineNumber() + ")");
                return true;
            }

            /** kamera ve karekod_okuyucu: getUserMedia için CAMERA izni. */
            @Override
            public void onPermissionRequest(PermissionRequest istek) {
                runOnUiThread(() -> {
                    if (checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
                        istek.grant(istek.getResources());
                    } else {
                        bekleyenKamera = istek;
                        requestPermissions(new String[] {Manifest.permission.CAMERA}, 2);
                    }
                });
            }

            /** konum: navigator.geolocation için konum izni. */
            @Override
            public void onGeolocationPermissionsShowPrompt(String kaynak, GeolocationPermissions.Callback geri) {
                if (checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED) {
                    geri.invoke(kaynak, true, false);
                } else {
                    bekleyenKonum = geri;
                    bekleyenKonumKaynagi = kaynak;
                    requestPermissions(new String[] {Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION}, 3);
                }
            }

            /** dosya_seç: sistemin dosya seçicisi. */
            @Override
            public boolean onShowFileChooser(WebView w, ValueCallback<Uri[]> geri, FileChooserParams ayar) {
                if (dosyaSecimi != null) dosyaSecimi.onReceiveValue(null);
                dosyaSecimi = geri;
                try {
                    startActivityForResult(ayar.createIntent(), 4);
                } catch (Exception e) {
                    dosyaSecimi = null;
                    return false;
                }
                return true;
            }
        });
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

            @Override
            public void onPageFinished(WebView w, String adres) {
                if (getIntent().getBooleanExtra("orhunca_sinama", false)) sinama();
            }
        });
        // Android 15'te uygulama kenardan kenara çizilir: sistem çubukları kadar boşluk bırakılır.
        // WebView kendi iç boşluğunu (padding) yok saydığı için boşluk bir kaba verilir.
        FrameLayout kap = new FrameLayout(this);
        kap.addView(sayfa, new FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT));
        if (Build.VERSION.SDK_INT >= 30) {
            kap.setOnApplyWindowInsetsListener((v, ic) -> {
                Insets i = ic.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.ime() | WindowInsets.Type.displayCutout());
                v.setPadding(i.left, i.top, i.right, i.bottom);
                return WindowInsets.CONSUMED;
            });
        }
        setContentView(kap);
        if (durum != null) sayfa.restoreState(durum);
        else sayfa.loadUrl("file:///android_asset/uygulama.html");
    }

    /**
     * Uçtan uca sınama (araclar/android_sinamasi.sh): sayfadaki yazı logcat'e yazılır
     * (etiket: OrhuncaSinama); `tikla` verildiyse o yazılı düğmeye basılıp yazı yeniden yazılır.
     */
    private void sinama() {
        Handler h = new Handler(Looper.getMainLooper());
        String tikla = getIntent().getStringExtra("tikla");
        h.postDelayed(() -> {
            yaziyiKaydet();
            if (tikla == null) return;
            String js = "(function(){for(const b of document.querySelectorAll('button')){if(b.textContent.trim()==="
                    + org.json.JSONObject.quote(tikla) + "){b.click();return 1}}return 0})()";
            sayfa.evaluateJavascript(js, v -> h.postDelayed(this::yaziyiKaydet, 1500));
        }, 3000);
    }

    private void yaziyiKaydet() {
        sayfa.evaluateJavascript("document.body.innerText", v -> {
            String metin = v;
            try {
                metin = new org.json.JSONArray("[" + v + "]").getString(0);
            } catch (Exception e) {
                // değer olduğu gibi yazılır
            }
            Log.i("OrhuncaSinama", metin.replace('\n', ' '));
        });
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

    @Override
    protected void onActivityResult(int kod, int sonuc, Intent veri) {
        super.onActivityResult(kod, sonuc, veri);
        if (kod == 4 && dosyaSecimi != null) {
            dosyaSecimi.onReceiveValue(WebChromeClient.FileChooserParams.parseResult(sonuc, veri));
            dosyaSecimi = null;
        }
    }

    @Override
    public void onRequestPermissionsResult(int kod, String[] izinler, int[] sonuclar) {
        super.onRequestPermissionsResult(kod, izinler, sonuclar);
        boolean verildi = sonuclar.length > 0 && sonuclar[0] == PackageManager.PERMISSION_GRANTED;
        if (kod == 2 && bekleyenKamera != null) {
            if (verildi) bekleyenKamera.grant(bekleyenKamera.getResources());
            else bekleyenKamera.deny();
            bekleyenKamera = null;
            return;
        }
        if (kod == 3 && bekleyenKonum != null) {
            bekleyenKonum.invoke(bekleyenKonumKaynagi, verildi, false);
            bekleyenKonum = null;
            return;
        }
        String[] b = bekleyenBildirim;
        bekleyenBildirim = null;
        if (kod == 1 && b != null && sonuclar.length > 0 && sonuclar[0] == PackageManager.PERMISSION_GRANTED) {
            bildirimGoster(b[0], b[1]);
        }
    }

    private void bildirimGoster(String baslik, String metin) {
        if (Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            // İzin verilince bu bildirim gösterilir; kullanıcının ikinci kez basması gerekmez.
            bekleyenBildirim = new String[] {baslik, metin};
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
