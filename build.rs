fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        embed_resource::compile("assets/icons/app.rc", embed_resource::NONE);
    }
}
