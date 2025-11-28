use crate::lexer::Token;
use crate::parser::parse;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Rich;
use chumsky::prelude::SimpleSpan;
use std::fs;

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
    let Ok(mut ast) = parse(src) else {
        diagnose(src, input_filename, parse(src).err().unwrap());
        return;
    };

    let Ok(_) = ast.check() else {
        diagnose(src, input_filename, ast.check().err().unwrap());
        return;
    };

    let str = serde_json::to_string(&ast).unwrap_or_default();
    match fs::write(format!("{output_dir}/{output_name}"), str) {
        Ok(_) => println!("{output_name} is compiled in {output_dir}"),
        Err(err) => {
            println!("errors while flushing {output_name} into {output_dir}, err={err}")
        }
    };
}
