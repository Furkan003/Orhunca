//! Windows'ta exe'ye Orhunca simgesi gömülür.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut r = winresource::WindowsResource::new();
        r.set_icon("simge/simge.ico");
        r.set("FileDescription", "Orhunca uygulaması");
        r.set("ProductName", "Orhunca");
        if let Err(e) = r.compile() {
            println!("cargo:warning=simge gömülemedi: {e}");
        }
    }
}
