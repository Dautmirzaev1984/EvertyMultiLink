fn main() {
    println!("cargo:rerun-if-changed=../../logo.png");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let image = image::open("../../logo.png")
        .expect("read application logo")
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    let path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("app.ico");
    let mut output = std::io::BufWriter::new(std::fs::File::create(&path).expect("create icon"));
    image
        .write_to(&mut output, image::ImageFormat::Ico)
        .expect("encode icon");
    drop(output);
    winresource::WindowsResource::new()
        .set_icon(path.to_str().expect("icon path"))
        .compile()
        .expect("embed Windows icon");
}
