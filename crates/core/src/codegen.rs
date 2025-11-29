use crate::lexer::Token;
use crate::parser::parse;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Rich;
use chumsky::prelude::SimpleSpan;
use std::fs;
use std::path::Path;

pub fn diagnose(src: &str, filename: &str, errs: Vec<Rich<Token, SimpleSpan>>) {
    for err in errs {
        Report::build(ReportKind::Error, (filename, err.span().into_range()))
            .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
            .with_message(err.to_string())
            .with_label(
                Label::new((filename, err.span().into_range()))
                    .with_message(err.reason().to_string())
                    .with_color(Color::Red),
            )
            .finish()
            .print((filename, Source::from(src)))
            .unwrap();
    }
}

pub fn compile(src: &str, input_filename: &str, output_dir: &str, output_name: &str) {
    let mut ast = match parse(input_filename, src) {
        Ok(ast) => ast,
        Err(errs) => {
            diagnose(src, input_filename, errs);
            return;
        }
    };

    if let Err(errs) = ast.check() {
        diagnose(src, input_filename, errs);
        return;
    }

    let json_output = match serde_json::to_string(&ast) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to serialized the produced ast err={}", e);
            return;
        }
    };

    let output_path = Path::new(output_dir).join(output_name);

    match fs::write(&output_path, json_output) {
        Ok(_) => println!("{output_name} is compiled in {output_dir}"),
        Err(err) => {
            eprintln!("error writing to {} err={}", output_path.display(), err)
        }
    };
}
