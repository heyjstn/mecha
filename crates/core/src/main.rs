use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    source: PathBuf,

    #[arg(short, long)]
    out: Option<PathBuf>,

    #[arg(short, long)]
    target: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let source_path = args.source;

    if !source_path.exists() {
        anyhow::bail!("source file doesn't exist: {}", source_path.display());
    }

    let current_dir = env::current_dir().context("failed to get current dir")?;
    let output_dir = args.out.unwrap_or(current_dir);

    let file_stem = source_path
        .file_stem()
        .context("invalid source filename")?
        .to_string_lossy();

    let output_filename = format!("{}.json", file_stem);
    let full_output_path = output_dir.join(&output_filename);

    println!("source = {}", source_path.display());
    println!("full_output_path = {}", full_output_path.display());

    let src = fs::read_to_string(&source_path)
        .with_context(|| format!("unable to read source file at {}", source_path.display()))?;

    let source_filename = source_path
        .file_name()
        .context("invalid source filename")?
        .to_string_lossy();

    let output_dir_str = output_dir.to_string_lossy();

    mecha_compiler::codegen::compile(&src, &source_filename, &output_dir_str, &output_filename);

    Ok(())
}
