use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(output)) = (args.next(), args.next()) else {
        eprintln!("usage: echo-codegen <input.yaml> <output.rs>");
        return ExitCode::from(2);
    };

    match run(&input, &output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("echo-codegen: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(input)?;
    let graph = echo_codegen::parse(&source)?;
    let code = echo_codegen::generate(&graph, input);
    std::fs::write(output, code)?;
    println!("generated {output} from {input}");
    Ok(())
}
