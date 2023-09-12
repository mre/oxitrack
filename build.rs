fn main() {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={timestamp}");

    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");
}
