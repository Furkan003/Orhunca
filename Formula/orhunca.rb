# Homebrew: brew tap furkan003/orhunca https://github.com/Furkan003/Orhunca
#           brew install orhunca
# Bu dosya üretilir: python3 araclar/dagitim_bildirimleri.py v1.0.0
class Orhunca < Formula
  desc "Türkçe programlama dili: derleyici, Orhunca Stüdyo ve dil sunucusu"
  homepage "https://furkan003.github.io/Orhunca/"
  version "1.0.0"
  license "MIT"

  on_macos do
    url "https://github.com/Furkan003/Orhunca/releases/download/v1.0.0/orhunca-macos.tar.gz"
    sha256 "b79ed01a52b409d4c3427560c61d1434f4c39e23d698d1c910fe69a3ab0be9d0"
  end

  on_linux do
    on_intel do
      url "https://github.com/Furkan003/Orhunca/releases/download/v1.0.0/orhunca-linux-x86_64.tar.gz"
      sha256 "69d3351e07d40e701aecc1cb54a1b351ac150905f126175170bc4f0502f7fbbf"
    end
    on_arm do
      url "https://github.com/Furkan003/Orhunca/releases/download/v1.0.0/orhunca-linux-aarch64.tar.gz"
      sha256 "c1344b799ec290874ae5b8dfc34534ef4ef4cd4ef2914285806e74765ae91e59"
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
