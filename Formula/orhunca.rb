# Homebrew: brew tap furkan003/orhunca https://github.com/Furkan003/Orhunca
#           brew install orhunca
# Bu dosya üretilir: python3 araclar/dagitim_bildirimleri.py v0.11.0
class Orhunca < Formula
  desc "Türkçe programlama dili: derleyici, Orhunca Stüdyo ve dil sunucusu"
  homepage "https://furkan003.github.io/Orhunca/"
  version "0.11.0"
  license "MIT"

  on_macos do
    url "https://github.com/Furkan003/Orhunca/releases/download/v0.11.0/orhunca-macos.tar.gz"
    sha256 "c902769363cda962f542361cfd91982892f4ba5f610afe5c79e7c7ac544de5e3"
  end

  on_linux do
    on_intel do
      url "https://github.com/Furkan003/Orhunca/releases/download/v0.11.0/orhunca-linux-x86_64.tar.gz"
      sha256 "e0f151519d44533e074685d8e84d4649461f3fe2c0d2220fdf957f74b471094e"
    end
    on_arm do
      url "https://github.com/Furkan003/Orhunca/releases/download/v0.11.0/orhunca-linux-aarch64.tar.gz"
      sha256 "4f472add3e09d95743176c7244e8dbd472eda83987a3d55b0797b1c2113a5d66"
    end
  end

  def install
    bin.install "orhunca"
  end

  def caveats
    <<~EOS
      Orhunca Stüdyo'yu açmak için: orhunca stüdyo
      macOS'ta program derlemek için Xcode komut satırı araçları gerekir: xcode-select --install
    EOS
  end

  test do
    (testpath/"selam.ohc").write("\"Merhaba\"'yı yaz.\n")
    assert_equal "Merhaba", shell_output("#{bin}/orhunca çalıştır selam.ohc").strip
  end
end
