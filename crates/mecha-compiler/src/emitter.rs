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

pub fn flush(src: &str, input_filename: &str, output_dir: &str, output_name: &str) -> () {
    match parse(src) {
        Err(errs) => {
            diagnose(src, input_filename, errs);
        }
        Ok(schema) => {
            let str = serde_json::to_string(&schema).unwrap_or("".to_string());
            match fs::write(format!("{output_dir}/{output_name}"), str) {
                Ok(_) => println!("{output_name} is compiled in {output_dir}"),
                Err(err) => {
                    println!("errors while flushing {output_name} into {output_dir}, err={err}")
                }
            };
        }
    }
}
