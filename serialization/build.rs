use std::process::Command;
use std::io::{self, Write};

fn main() {
    let output = Command::new("python3")
        .arg("./build.py")
        .output()
        .expect("Failed to execute Python script");

    if output.status.success() {
        io::stdout().write_all(&output.stdout).unwrap();
    } else {
        io::stderr().write_all(&output.stderr).unwrap();
        panic!("Python script failed to run");
    }
}
