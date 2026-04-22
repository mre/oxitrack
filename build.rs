use std::path::Path;

fn main() {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={timestamp}");

    if !Path::new("dev").exists() {
        println!("cargo:rustc-env=SQLX_OFFLINE=false");
    }

    // Re-run only when the files that affect cache-busting actually change.
    // Without these directives Cargo would re-run build.rs on every build,
    // producing a new BUILD_TIMESTAMP and busting browser caches unnecessarily.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=templates");
}
