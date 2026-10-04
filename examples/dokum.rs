//! Bir .ohc dosyasının ayrıştırma dökümünü yazar (öz/ayrıştırıcı.ohc ile karşılaştırmak için).
fn main() {
    let yol = std::env::args().nth(1).expect("dosya");
    let kaynak = std::fs::read_to_string(yol).unwrap();
    print!("{}", orhunca::dokum::kaynak(&kaynak));
}
