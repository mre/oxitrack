fn main() {
    // Add the hash of the HEAD commit as a compile time defined environment variable.
    let repo = gix::open(".").unwrap();
    let head_id = repo.head_id().unwrap();
    let git_hash = head_id.to_hex();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");
}
