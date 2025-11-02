use std::{fs, process::Command};

fn main() {
    let files: Vec<String> = fs::read_dir("resources/protocol")
        .unwrap()
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.extension()?.to_str()? == "fbs" {
                Some(path.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();

    let mut cmd = Command::new("flatc");
    cmd.args([
        "--rust",
        "-o", "src/protocol/generated",
    ]);
    cmd.args(&files);

    let status = cmd.status().expect("failed to run flatc");
    if !status.success() {
        panic!("flatc failed");
    }
}
