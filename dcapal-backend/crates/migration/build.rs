//! Rebuilds the crate when a migration file is added or changed, because
//! `sqlx::migrate!` embeds the directory contents at compile time.

fn main() {
    println!("cargo:rerun-if-changed=../../migrations");
}
