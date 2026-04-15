use std::path::Path;

fn main() {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={timestamp}");

    if !Path::new("dev").exists() {
        println!("cargo:rustc-env=SQLX_OFFLINE=false");
    }

    // Trigger recompilation
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=templates");
}
