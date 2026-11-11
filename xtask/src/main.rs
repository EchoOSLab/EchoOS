use std::process::Command;

fn run_script(script: &str) -> ! {
    let status = Command::new("bash")
        .arg(script)
        .status()
        .unwrap_or_else(|error| panic!("failed to spawn {script}: {error}"));
    std::process::exit(status.code().unwrap_or(1));
}

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    match cmd.as_str() {
        "hello" | "" => run_script("tools/scripts/qemu-hello.sh"),
        "graph" => run_script("tools/scripts/qemu-graph.sh"),
        "qemu-runtime" => run_script("tools/scripts/qemu-runtime.sh"),
        other => {
            eprintln!("unknown xtask: {other}");
            eprintln!("usage: cargo xtask hello|graph|qemu-runtime");
            std::process::exit(2);
        }
    }
}
