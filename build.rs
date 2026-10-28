// The exe's icon, and the name Windows shows for it in "Open with" and the Task Manager.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon/icon.ico")
            .set("FileDescription", "DystoPad")
            .set("ProductName", "DystoPad")
            .set("CompanyName", "YodelingGoethe")
            .set("LegalCopyright", "GPL-3.0-or-later");
        res.compile().expect("Windows resources");
    }
}
