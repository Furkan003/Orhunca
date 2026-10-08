# Homebrew: brew tap furkan003/orhunca https://github.com/Furkan003/Orhunca
#           brew install orhunca
# Bu dosya üretilir: python3 araclar/dagitim_bildirimleri.py v0.9.0
class Orhunca < Formula
  desc "Türkçe programlama dili: derleyici, Orhunca Stüdyo ve dil sunucusu"
  homepage "https://furkan003.github.io/Orhunca/"
  version "0.9.0"
  license "MIT"

  on_macos do
    url "https://github.com/Furkan003/Orhunca/releases/download/v0.9.0/orhunca-macos.tar.gz"
    sha256 "bba4f2df95748b8d41406630477c4f10dee27e51d8aac1f6ac5d7d93d7c3db5a"
  end

  on_linux do
    on_intel do
      url "https://github.com/Furkan003/Orhunca/releases/download/v0.9.0/orhunca-linux-x86_64.tar.gz"
      sha256 "fc6875f9abd9c0ee70f8e40f9aa48eaf9c94e21544f45f76d1dcd50debf6a112"
    end
    on_arm do
      url "https://github.com/Furkan003/Orhunca/releases/download/v0.9.0/orhunca-linux-aarch64.tar.gz"
      sha256 "533f41b5e398997cb63fb8a618fe584635041be16f3b12e2eef28dcf754566dd"
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
