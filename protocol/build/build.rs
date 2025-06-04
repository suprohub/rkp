use std::{env, fs, path::Path, process::Command};

use proc_macro2::TokenStream;

mod packet_id;

fn main() {
    write_generated_file(packet_id::build(), "packet_id.rs");
}

pub fn write_generated_file(content: TokenStream, out_file: &str) {
    let out_dir = env::var_os("OUT_DIR").expect("failed to get OUT_DIR env var");
    let path = Path::new(&out_dir).join(out_file);
    let code = content.to_string();

    fs::write(&path, code).expect("Failed to write to fs");

    // Try to format the output for debugging purposes.
    // Doesn't matter if rustfmt is unavailable.
    let _ = Command::new("rustfmt").arg(path).output();
}
