use clap::{Arg, Command};
use std::env;

fn main() {
    let matches = Command::new("mecha-compiler")
        .arg(
            Arg::new("source")
                .short('s')
                .long("source")
                .required(true)
                .help("the dir of source .schema file"),
        )
        .arg(
            Arg::new("out")
                .short('o')
                .long("out")
                .required(false)
                .help("the dir of output .json file"),
        )
        .arg(
            Arg::new("target")
                .short('t')
                .long("target")
                .required(false)
                .help("the sql type target, valids are pgsql, mysql"),
        )
        .get_matches();

    let source = matches
        .get_one::<String>("source")
        .expect("source file must not empty");

    let cur_dir = env::current_dir().unwrap();
    let default_output_dir_str = &cur_dir.to_str().unwrap().to_string();

    let source_path_segments: Vec<&str> = source.split("/").collect();
    let default_output_filename_str = *source_path_segments.last().unwrap();

    let out = matches
        .get_one::<String>("out")
        .unwrap_or(default_output_dir_str);

    println!("source = {source}");
    println!("full_output_path = {default_output_dir_str}/{default_output_filename_str}")
}
