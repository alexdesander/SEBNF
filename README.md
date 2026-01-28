# SEBNF

**SEBNF** ("Simpler EBNF") is a simple way
to define a language's grammar.

I created this because I, like many others,
was not satisfied with the common standards
and wanted to write a tool that tells me if
a grammar is LL(1) (supporting regex!).

Relevant xkcd: https://xkcd.com/927/

---

## SEBNF of SEBNF

The best overview of SEBNF is its own grammar:
```txt
(*                 SEBNF of SEBNF                 *)
(*  What you are reading right now is a comment.  *)
(*  SEBNF stands for "Simpler EBNF".              *)
(*  Whitespaces, newlines and comments ignored.   *)
(*  Regex variant must be: Rust "regex" crate.    *)
(*  The first rule is implied to be the start.    *)

(*  IMPORTANT:
        "|" (choice operator) is only allowed inside
        () or at the start of a new rule right side.
        Valid:   A := B | C { ("a" | "b") }.
        Invalid: A := B | C { "a" | "b" }.
 *)

grammar      := { rule }.
rule         := non_terminal ":=" alternatives ".".
alternatives := { item } { "|" { item } }.
item         := non_terminal
              | terminal
              | regex
              | "[" { item } "]"
              | "{" { item } "}"
              | "(" { item } { "|" { item } } ")"
              .
non_terminal := /[0-9A-Za-z_]+/.
terminal     := /"(?:[^"\\]|\\.)*"/.
regex        := /\/(?:[^\/\\]|\\.)*?\//.
_comment     := /\(\*[\s\S]*?\*\)/.

```

The only feature not mentioned above is the
empty (epsilon):
```txt
A := something
   |
   .
```

---

## CLI Tool

This repository is a Rust crate. It contains the source
of the SEBNF CLI tool which is capable of:
- Parsing an SEBNF
- Converting it to a BNF (SEBNF without {}, [], ())
- Extracting the FIRST and FOLLOW sets of a grammar
- Checking if a grammar is LL(1) (Yes, it works with regex!)

### CLI Tool Usage

1. Build with `cargo build --release` from source (you need Rust/Cargo)
2. `./sebnf_tool is-ll1 < sebnf_of_sebnf.txt`
3. `./sebnf_tool help`

Or, if you are in the repo:

4. `cargo run -- is-ll1 < sebnf_of_sebnf.txt`

---

## AI Usage

AI was *not* used to define SEBNF
or write the SEBNF of SEBNF.

LLMs were used to:
- Finish implementing parts of the
  recursive descent parser
- Implement pretty printing (`fmt` trait)
- Code review
- Fix typos in this README
- Implement error reporting using `miette`
- Test cases for regex_intersect
- Research
