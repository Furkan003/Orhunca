#!/bin/sh
# Orhunca'yı (orhunca komutu) kurar: Linux (x86-64, ARM64) ve macOS.
#   curl -fsSL https://furkan003.github.io/Orhunca/kur.sh | sh
# Kurulum yeri: ~/.local/bin (ORHUNCA_KURULUM ile değiştirilebilir). Yönetici izni gerekmez.
set -eu
DEPO="Furkan003/Orhunca"
HEDEF="${ORHUNCA_KURULUM:-$HOME/.local/bin}"

case "$(uname -s)" in
    Linux)
        case "$(uname -m)" in
            x86_64 | amd64) DOSYA="orhunca-linux-x86_64.tar.gz" ;;
            aarch64 | arm64)
                DOSYA="orhunca-linux-aarch64.tar.gz"
                command -v cc >/dev/null 2>&1 || echo "Not: ARM'da programları derlemek için bir C derleyicisi gerekir (sudo apt install gcc)." >&2 ;;
            *) echo "Bu işlemci desteklenmiyor: $(uname -m) (x86-64 ve ARM64 desteklenir)." >&2; exit 1 ;;
        esac ;;
    Darwin) DOSYA="orhunca-macos.tar.gz" ;;
    *) echo "Bu sistem desteklenmiyor: $(uname -s). Windows için: irm https://furkan003.github.io/Orhunca/kur.ps1 | iex" >&2; exit 1 ;;
esac

ADRES="https://github.com/$DEPO/releases/latest/download/$DOSYA"
GECICI=$(mktemp -d)
trap 'rm -rf "$GECICI"' EXIT
echo "İndiriliyor: $ADRES"
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$ADRES" -o "$GECICI/$DOSYA"
else
    wget -q "$ADRES" -O "$GECICI/$DOSYA"
fi
tar -xzf "$GECICI/$DOSYA" -C "$GECICI"
mkdir -p "$HEDEF"
install -m 755 "$GECICI/orhunca" "$HEDEF/orhunca"
echo "Kuruldu: $HEDEF/orhunca ($("$HEDEF/orhunca" sürüm))"

case ":$PATH:" in
    *":$HEDEF:"*) ;;
    *)
        SATIR="export PATH=\"$HEDEF:\$PATH\""
        for RC in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
            [ -f "$RC" ] && ! grep -qs "$HEDEF" "$RC" && printf '\n# Orhunca\n%s\n' "$SATIR" >> "$RC"
        done
        echo "Yeni bir terminal açın (ya da: $SATIR)" ;;
esac
echo "Başlamak için: orhunca stüdyo"
