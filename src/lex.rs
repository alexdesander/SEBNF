use std::fmt;

use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f]+")]
#[logos(skip r"\(\*[^*]*\*+(?:[^)*][^*]*\*+)*\)")]
pub enum Token {
    #[token(".")]
    Dot,
    #[token(":=")]
    Assign,
    #[token("(")]
    BracketRoundOpen,
    #[token(")")]
    BracketRoundClose,
    #[token("[")]
    BracketSquareOpen,
    #[token("]")]
    BracketSquareClose,
    #[token("{")]
    BracketCurlyOpen,
    #[token("}")]
    BracketCurlyClose,
    #[token("|")]
    Separator,

    #[regex(r"\r?\n")]
    NewLine,
    #[regex(r#"[0-9A-Za-z_]+"#, |lex| lex.slice().to_string())]
    NonTerminal(String),
    #[regex(r#""(?:[^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    Terminal(String),
    #[regex(r"\/(?:[^\/\\]|\\.)*?\/", |lex| lex.slice().to_string())]
    Regex(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Dot => write!(f, "'.'"),
            Token::Assign => write!(f, "':='"),
            Token::BracketRoundOpen => write!(f, "'('"),
            Token::BracketRoundClose => write!(f, "')'"),
            Token::BracketSquareOpen => write!(f, "'['"),
            Token::BracketSquareClose => write!(f, "']'"),
            Token::BracketCurlyOpen => write!(f, "'{{'"),
            Token::BracketCurlyClose => write!(f, "'}}'"),
            Token::Separator => write!(f, "'|'"),
            Token::NewLine => write!(f, "newline"),
            Token::NonTerminal(s) => write!(f, "non-terminal '{}'", s),
            Token::Terminal(s) => write!(f, "terminal \"{}\"", s),
            Token::Regex(s) => write!(f, "regex /{}/", s),
        }
    }
}
