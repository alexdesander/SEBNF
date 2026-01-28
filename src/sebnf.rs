#![allow(unused)]

use std::fmt;
use std::ops::Range;

use indexmap::IndexMap;
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

use crate::bnf::Bnf;
use crate::converter;
use crate::lex::Token;

fn to_source_span(span: &Range<usize>) -> SourceSpan {
    SourceSpan::new(span.start.into(), span.len().into())
}

#[derive(Error, Debug, Diagnostic)]
pub enum ParseError {
    #[error("unexpected token")]
    #[diagnostic(code(sebnf::unexpected_token))]
    UnexpectedToken {
        expected: String,
        found: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("expected {expected}, found {found}")]
        span: SourceSpan,
    },

    #[error("unexpected end of input, expected {expected}")]
    #[diagnostic(code(sebnf::unexpected_eof))]
    UnexpectedEof {
        expected: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("expected {expected}")]
        span: SourceSpan,
    },

    #[error("expected non-terminal at start of rule")]
    #[diagnostic(
        code(sebnf::expected_nonterminal),
        help("rules must start with a non-terminal identifier")
    )]
    ExpectedNonTerminal {
        found: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("expected a non-terminal here")]
        span: SourceSpan,
    },

    #[error("lexer error")]
    #[diagnostic(code(sebnf::lex_error))]
    LexError {
        #[source_code]
        src: NamedSource<String>,
        #[label("unrecognized token")]
        span: SourceSpan,
    },
}

#[derive(Debug)]
pub struct Sebnf {
    pub rules: IndexMap<String, Vec<Vec<Item>>>,
}

#[derive(Debug)]
pub enum Item {
    NonTerminal(String),
    Terminal(String),
    Regex(String),
    Optional(Vec<Item>),
    AnyAmount(Vec<Item>),
    Choice(Vec<Vec<Item>>),
}

struct Parser {
    tokens: Vec<(Token, Range<usize>)>,
    pos: usize,
    source: String,
    source_name: String,
}

impl Parser {
    fn new(tokens: Vec<(Token, Range<usize>)>, source: String, source_name: String) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
            source_name,
        }
    }

    fn named_source(&self) -> NamedSource<String> {
        NamedSource::new(&self.source_name, self.source.clone())
    }

    fn current_span(&self) -> Range<usize> {
        if let Some((_, span)) = self.tokens.get(self.pos) {
            span.clone()
        } else {
            let end = self.source.len();
            end..end
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn advance(&mut self) -> Option<(&Token, &Range<usize>)> {
        let item = self.tokens.get(self.pos);
        self.pos += 1;
        item.map(|(t, s)| (t, s))
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        match self.advance() {
            Some((tok, _)) if tok == expected => Ok(()),
            Some((tok, span)) => {
                let span = span.clone();
                Err(ParseError::UnexpectedToken {
                    expected: expected.to_string(),
                    found: tok.to_string(),
                    src: self.named_source(),
                    span: to_source_span(&span),
                })
            }
            None => Err(ParseError::UnexpectedEof {
                expected: expected.to_string(),
                src: self.named_source(),
                span: to_source_span(&self.current_span()),
            }),
        }
    }

    fn skip_newlines(&mut self) {
        while let Some(Token::NewLine) = self.peek() {
            self.advance();
        }
    }

    fn parse_grammar(&mut self) -> Result<Sebnf, ParseError> {
        let mut rules = IndexMap::new();

        self.skip_newlines();
        while self.peek().is_some() {
            let (name, alts) = self.parse_rule()?;
            rules.insert(name, alts);
            self.skip_newlines();
        }

        Ok(Sebnf { rules })
    }

    fn parse_rule(&mut self) -> Result<(String, Vec<Vec<Item>>), ParseError> {
        self.skip_newlines();
        let name = match self.advance() {
            Some((Token::NonTerminal(s), _)) => s.clone(),
            Some((tok, span)) => {
                let span = span.clone();
                return Err(ParseError::ExpectedNonTerminal {
                    found: tok.to_string(),
                    src: self.named_source(),
                    span: to_source_span(&span),
                });
            }
            None => {
                return Err(ParseError::UnexpectedEof {
                    expected: "non-terminal".to_string(),
                    src: self.named_source(),
                    span: to_source_span(&self.current_span()),
                });
            }
        };

        self.skip_newlines();
        self.expect(&Token::Assign)?;
        self.skip_newlines();
        let alts = self.parse_alternatives()?;
        self.skip_newlines();
        self.expect(&Token::Dot)?;

        Ok((name, alts))
    }

    fn parse_alternatives(&mut self) -> Result<Vec<Vec<Item>>, ParseError> {
        self.skip_newlines();
        let mut alternatives = vec![self.parse_items()?];

        self.skip_newlines();
        while let Some(Token::Separator) = self.peek() {
            self.advance();
            self.skip_newlines();
            alternatives.push(self.parse_items()?);
            self.skip_newlines();
        }

        Ok(alternatives)
    }

    fn parse_items(&mut self) -> Result<Vec<Item>, ParseError> {
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            match self.parse_item()? {
                Some(item) => items.push(item),
                None => break,
            }
        }
        Ok(items)
    }

    fn parse_item(&mut self) -> Result<Option<Item>, ParseError> {
        let Some(tok) = self.peek() else {
            return Ok(None);
        };

        match tok {
            Token::NonTerminal(_) => {
                if let Some((Token::NonTerminal(s), _)) = self.advance() {
                    Ok(Some(Item::NonTerminal(s.clone())))
                } else {
                    unreachable!()
                }
            }
            Token::Terminal(_) => {
                if let Some((Token::Terminal(s), _)) = self.advance() {
                    Ok(Some(Item::Terminal(s.clone())))
                } else {
                    unreachable!()
                }
            }
            Token::Regex(_) => {
                if let Some((Token::Regex(s), _)) = self.advance() {
                    Ok(Some(Item::Regex(s.clone())))
                } else {
                    unreachable!()
                }
            }
            Token::BracketSquareOpen => {
                self.advance();
                self.skip_newlines();
                let items = self.parse_items()?;
                self.skip_newlines();
                self.expect(&Token::BracketSquareClose)?;
                Ok(Some(Item::Optional(items)))
            }
            Token::BracketCurlyOpen => {
                self.advance();
                self.skip_newlines();
                let items = self.parse_items()?;
                self.skip_newlines();
                self.expect(&Token::BracketCurlyClose)?;
                Ok(Some(Item::AnyAmount(items)))
            }
            Token::BracketRoundOpen => {
                self.advance();
                self.skip_newlines();
                let alts = self.parse_alternatives()?;
                self.skip_newlines();
                self.expect(&Token::BracketRoundClose)?;
                if alts.len() == 1 {
                    let mut items = alts.into_iter().next().unwrap();
                    if items.len() == 1 {
                        Ok(Some(items.remove(0)))
                    } else {
                        Ok(Some(Item::Choice(vec![items])))
                    }
                } else {
                    Ok(Some(Item::Choice(alts)))
                }
            }
            _ => Ok(None),
        }
    }
}

impl Sebnf {
    pub fn parse(
        tokens: Vec<(Token, Range<usize>)>,
        source: String,
        source_name: impl Into<String>,
    ) -> Result<Self, ParseError> {
        let mut parser = Parser::new(tokens, source, source_name.into());
        parser.parse_grammar()
    }

    pub fn to_bnf(&self) -> Bnf {
        converter::sebnf_to_bnf(self)
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::NonTerminal(s) => write!(f, "{}", s),
            Item::Terminal(s) => write!(f, "{}", s),
            Item::Regex(s) => write!(f, "{}", s),
            Item::Optional(items) => {
                write!(f, "[")?;
                for item in items {
                    write!(f, " {}", item)?;
                }
                write!(f, " ]")
            }
            Item::AnyAmount(items) => {
                write!(f, "{{")?;
                for item in items {
                    write!(f, " {}", item)?;
                }
                write!(f, " }}")
            }
            Item::Choice(alts) => {
                write!(f, "(")?;
                for (i, alt) in alts.iter().enumerate() {
                    if i > 0 {
                        write!(f, " |")?;
                    }
                    for item in alt {
                        write!(f, " {}", item)?;
                    }
                }
                write!(f, " )")
            }
        }
    }
}

fn write_items(f: &mut fmt::Formatter<'_>, items: &[Item]) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }
        write!(f, "{}", item)?;
    }
    Ok(())
}

impl fmt::Display for Sebnf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let max_len = self.rules.keys().map(|k| k.len()).max().unwrap_or(0);

        for (name, alts) in &self.rules {
            if alts.len() == 1 {
                write!(f, "{:width$} := ", name, width = max_len)?;
                write_items(f, &alts[0])?;
                writeln!(f, ".")?;
            } else {
                let indent = " ".repeat(max_len + 2);

                write!(f, "{:width$} := ", name, width = max_len)?;
                write_items(f, &alts[0])?;
                writeln!(f)?;

                for alt in &alts[1..] {
                    write!(f, "{}| ", indent)?;
                    write_items(f, alt)?;
                    writeln!(f)?;
                }
                writeln!(f, "{}.", indent)?;
            }
        }

        Ok(())
    }
}
