# Homebrew: brew tap furkan003/orhunca https://github.com/Furkan003/Orhunca
#           brew install orhunca
# Bu dosya üretilir: python3 araclar/dagitim_bildirimleri.py v0.10.0
class Orhunca < Formula
  desc "Türkçe programlama dili: derleyici, Orhunca Stüdyo ve dil sunucusu"
  homepage "https://furkan003.github.io/Orhunca/"
  version "0.10.0"
  license "MIT"

  on_macos do
    url "https://github.com/Furkan003/Orhunca/releases/download/v0.10.0/orhunca-macos.tar.gz"
    sha256 "1cee49339c97824e950789c538438dd66c435fab1439639bb1a59d9c81a61b20"
  end

  on_linux do
    on_intel do
      url "https://github.com/Furkan003/Orhunca/releases/download/v0.10.0/orhunca-linux-x86_64.tar.gz"
      sha256 "2f5011524de5213205a8d40d6180b799f0b05ff7b8f98e08c24823eb293df32e"
    end
    on_arm do
      url "https://github.com/Furkan003/Orhunca/releases/download/v0.10.0/orhunca-linux-aarch64.tar.gz"
      sha256 "3b65d14542e4a530cb78aa96d5ff586e58ca81da8e9c2d07d6111e50d05d7028"
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
