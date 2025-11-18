use std::fmt::{Display, Formatter};
use logos::Logos;
use chumsky::input::{Input, Stream, ValueInput};
use chumsky::prelude::SimpleSpan;

#[derive(Logos, Clone, PartialEq)]
#[derive(Debug)]
pub enum Token<'a> {
    Err,

    #[token("abstract")]
    Abstract,
    #[token("table")]
    Table,
    #[token("extends")]
    Extends,

    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*")]
    Id(&'a str),

    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,

    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,

    #[token("primary")]
    Primary,
    #[token("unique")]
    Unique,

    #[token("ref")]
    Ref,
    #[token("=>")]
    RefOneToMany,
    #[token("==")]
    RefOneToOne,
    #[token("<>")]
    RefManyToMany,

    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,

    #[token("indexes")]
    Indexes,

    #[regex(r"[ \t\f\n]+", logos::skip)]
    Whitespace,
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Err => write!(f, "<error>"),
            Token::Abstract => write!(f, "abstract"),
            Token::Table => write!(f, "table"),
            Token::Extends => write!(f, "extends"),
            Token::Id(name) => write!(f, "Id<{name}>"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::Primary => write!(f, "primary"),
            Token::Unique => write!(f, "unique"),
            Token::Ref => write!(f, "ref"),
            Token::RefOneToMany => write!(f, "=>"),
            Token::RefOneToOne => write!(f, "=="),
            Token::RefManyToMany => write!(f, "<>"),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Indexes => write!(f, "indexes"),
            Token::Whitespace => write!(f, "<whitespace>"),
            _ => write!(f, "<unexisted>"),
        }
    }
}

pub fn tokenize(src: &'_ str) -> impl ValueInput<'_, Token=Token<'_>, Span=SimpleSpan> {
    let token_iter = Token::lexer(src).spanned().map(|(tok, span)| match tok {
        Ok(tok) => {
            let simple_span: SimpleSpan = span.into();
            (tok, simple_span)
        }
        Err(()) => (Token::Err, span.into()),
    });

    Stream::from_iter(token_iter).map((0..src.len()).into(), |(t, s): (_, _)| (t, s))
}