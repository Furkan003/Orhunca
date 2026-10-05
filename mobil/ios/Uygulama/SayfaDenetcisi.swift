import UIKit
import WebKit
import AudioToolbox
import UserNotifications

/// Orhunca arayüz programını (uygulama.html) tam ekran gösterir; telefon komutlarını
/// (titret, paylaş, bildirim_gönder) `webkit.messageHandlers.orhunca` kanalından alır.
class SayfaDenetcisi: UIViewController, WKScriptMessageHandler, WKNavigationDelegate, WKUIDelegate {
    private var sayfa: WKWebView!

    override func loadView() {
        let ayar = WKWebViewConfiguration()
        ayar.userContentController.add(self, name: "orhunca")
        ayar.preferences.javaScriptCanOpenWindowsAutomatically = false
        sayfa = WKWebView(frame: .zero, configuration: ayar)
        sayfa.navigationDelegate = self
        sayfa.uiDelegate = self
        sayfa.allowsBackForwardNavigationGestures = true
        view = sayfa
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        if let adres = Bundle.main.url(forResource: "uygulama", withExtension: "html") {
            sayfa.loadFileURL(adres, allowingReadAccessTo: adres.deletingLastPathComponent())
        }
    }

    func userContentController(_ denetci: WKUserContentController, didReceive ileti: WKScriptMessage) {
        guard let v = ileti.body as? [String: Any], let tur = v["tur"] as? String else { return }
        switch tur {
        case "titret":
            let ms = (v["ms"] as? Double) ?? 200
            if ms < 150 {
                UIImpactFeedbackGenerator(style: .medium).impactOccurred()
            } else {
                AudioServicesPlaySystemSound(kSystemSoundID_Vibrate)
            }
        case "paylas":
            let pencere = UIActivityViewController(activityItems: [(v["metin"] as? String) ?? ""], applicationActivities: nil)
            pencere.popoverPresentationController?.sourceView = view
            present(pencere, animated: true)
        case "bildirim":
            bildirimGoster(baslik: (v["baslik"] as? String) ?? "", metin: (v["metin"] as? String) ?? "")
        default:
            break
        }
    }

    private func bildirimGoster(baslik: String, metin: String) {
        let merkez = UNUserNotificationCenter.current()
        merkez.requestAuthorization(options: [.alert, .sound]) { izin, _ in
            guard izin else { return }
            let icerik = UNMutableNotificationContent()
            icerik.title = baslik
            icerik.body = metin
            icerik.sound = .default
            let tetik = UNTimeIntervalNotificationTrigger(timeInterval: 1, repeats: false)
            merkez.add(UNNotificationRequest(identifier: UUID().uuidString, content: icerik, trigger: tetik))
        }
    }

    // Dış bağlantılar Safari'de açılır
    func webView(_ w: WKWebView, decidePolicyFor eylem: WKNavigationAction, decisionHandler karar: @escaping (WKNavigationActionPolicy) -> Void) {
        if let u = eylem.request.url, u.scheme == "http" || u.scheme == "https" {
            UIApplication.shared.open(u)
            karar(.cancel)
            return
        }
        karar(.allow)
    }

    // alert(), confirm() ve prompt() (oku()) pencereleri
    func webView(_ w: WKWebView, runJavaScriptAlertPanelWithMessage mesaj: String, initiatedByFrame _: WKFrameInfo, completionHandler bitti: @escaping () -> Void) {
        let p = UIAlertController(title: nil, message: mesaj, preferredStyle: .alert)
        p.addAction(UIAlertAction(title: "Tamam", style: .default) { _ in bitti() })
        present(p, animated: true)
    }

    func webView(_ w: WKWebView, runJavaScriptConfirmPanelWithMessage mesaj: String, initiatedByFrame _: WKFrameInfo, completionHandler bitti: @escaping (Bool) -> Void) {
        let p = UIAlertController(title: nil, message: mesaj, preferredStyle: .alert)
        p.addAction(UIAlertAction(title: "Vazgeç", style: .cancel) { _ in bitti(false) })
        p.addAction(UIAlertAction(title: "Tamam", style: .default) { _ in bitti(true) })
        present(p, animated: true)
    }

    func webView(_ w: WKWebView, runJavaScriptTextInputPanelWithPrompt soru: String, defaultText varsayilan: String?, initiatedByFrame _: WKFrameInfo, completionHandler bitti: @escaping (String?) -> Void) {
        let p = UIAlertController(title: nil, message: soru, preferredStyle: .alert)
        p.addTextField { $0.text = varsayilan }
        p.addAction(UIAlertAction(title: "Vazgeç", style: .cancel) { _ in bitti(nil) })
        p.addAction(UIAlertAction(title: "Tamam", style: .default) { _ in bitti(p.textFields?.first?.text) })
        present(p, animated: true)
    }
}
