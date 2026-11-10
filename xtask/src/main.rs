use std::process::Command;

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    match cmd.as_str() {
        "hello" | "" => {
            let status = Command::new("bash")
                .arg("tools/scripts/qemu-hello.sh")
                .status()
                .expect("failed to spawn qemu-hello.sh");
            std::process::exit(status.code().unwrap_or(1));
        }
        other => {
            eprintln!("unknown xtask: {other}");
            eprintln!("usage: cargo xtask hello");
            std::process::exit(2);
        }
    }
}
