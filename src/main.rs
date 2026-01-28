use std::io::Read;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use miette::NamedSource;

use crate::lex::Token;
use crate::sebnf::{ParseError, Sebnf};
use crate::sets::Ll1Error;
use logos::Logos;

pub mod bnf;
pub mod converter;
pub mod lex;
pub mod regex_intersect;
pub mod sebnf;
pub mod sets;

#[derive(Parser)]
#[command(name = "ebnf_set_calc")]
#[command(about = "EBNF grammar analysis tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate SEBNF syntax
    Validate,
    /// Convert SEBNF to BNF
    ToBnf,
    /// Extract FIRST and FOLLOW sets
    ExtractSets,
    /// Check if grammar is LL(1)
    IsLl1,
}

fn read_stdin() -> String {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read from stdin");
    input
}

fn parse_sebnf(input: &str) -> Result<Sebnf, ParseError> {
    let tokens: Result<Vec<_>, _> = Token::lexer(input)
        .spanned()
        .map(|(res, span)| res.map(|t| (t, span.clone())).map_err(|_| span))
        .collect();

    let tokens = match tokens {
        Ok(t) => t,
        Err(span) => {
            return Err(ParseError::LexError {
                src: NamedSource::new("<stdin>", input.to_string()),
                span: (span.start, span.len()).into(),
            });
        }
    };

    let sebnf = Sebnf::parse(tokens, input.to_string(), "<stdin>")?;
    sebnf.validate(input.to_string(), "<stdin>")?;
    Ok(sebnf)
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
enum CliError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Ll1(#[from] Ll1Error),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let input = read_stdin();

    let result: Result<ExitCode, CliError> = (|| match cli.command {
        Commands::Validate => {
            parse_sebnf(&input)?;
            println!("Valid SEBNF");
            Ok(ExitCode::SUCCESS)
        }
        Commands::ToBnf => {
            let sebnf = parse_sebnf(&input)?;
            let bnf = sebnf.to_bnf();
            print!("{}", bnf);
            Ok(ExitCode::SUCCESS)
        }
        Commands::ExtractSets => {
            let sebnf = parse_sebnf(&input)?;
            let bnf = sebnf.to_bnf();
            let sets = bnf.first_and_follow_sets();
            print!("{}", sets);
            Ok(ExitCode::SUCCESS)
        }
        Commands::IsLl1 => {
            let sebnf = parse_sebnf(&input)?;
            let bnf = sebnf.to_bnf();
            let result = bnf.is_ll1()?;
            print!("{}", result);
            if result.is_ll1() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::FAILURE)
            }
        }
    })();

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{:?}", miette::Report::new(e));
            ExitCode::FAILURE
        }
    }
}
