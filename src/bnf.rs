use indexmap::IndexMap;
use std::fmt;

use crate::sets::{
    Ll1Conflict, Ll1ConflictKind, Ll1Error, Ll1Result, Sets, extract_sets, find_set_conflicts,
    first_of_sequence,
};

#[derive(Debug)]
pub struct Bnf {
    pub rules: IndexMap<String, Vec<Vec<Item>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    NonTerminal(String),
    Terminal(String),
    Regex(String),
}

impl Bnf {
    pub fn first_and_follow_sets(&self) -> Sets {
        extract_sets(self)
    }

    /// Checks if the grammar is LL(1) and returns detailed information
    pub fn is_ll1(&self) -> Result<Ll1Result, Ll1Error> {
        let sets = self.first_and_follow_sets();
        let mut conflicts = Vec::new();

        for (nt, productions) in &self.rules {
            // Skip non-terminals with only one production
            if productions.len() <= 1 {
                continue;
            }

            // Compute FIRST set for each production: (first_set_without_epsilon, is_nullable)
            let prod_firsts: Vec<_> = productions
                .iter()
                .map(|prod| first_of_sequence(prod, &sets.first))
                .collect();

            // Check FIRST/FIRST conflicts between all pairs of productions
            for i in 0..productions.len() {
                for j in (i + 1)..productions.len() {
                    let item_conflicts = find_set_conflicts(&prod_firsts[i].0, &prod_firsts[j].0)?;

                    if !item_conflicts.is_empty() {
                        conflicts.push(Ll1Conflict {
                            non_terminal: nt.clone(),
                            kind: Ll1ConflictKind::FirstFirst {
                                production1: productions[i].clone(),
                                production2: productions[j].clone(),
                            },
                            conflicts: item_conflicts,
                        });
                    }
                }
            }

            // Check FIRST/FOLLOW conflicts
            // If production i is nullable, check if FIRST of other productions
            // conflicts with FOLLOW(nt)
            let follow_set = sets.follow.get(nt).cloned().unwrap_or_default();

            for i in 0..productions.len() {
                // Check if production i is nullable
                if prod_firsts[i].1 {
                    for j in 0..productions.len() {
                        if i != j {
                            let item_conflicts =
                                find_set_conflicts(&prod_firsts[j].0, &follow_set)?;

                            if !item_conflicts.is_empty() {
                                conflicts.push(Ll1Conflict {
                                    non_terminal: nt.clone(),
                                    kind: Ll1ConflictKind::FirstFollow {
                                        nullable_production: productions[i].clone(),
                                        other_production: productions[j].clone(),
                                    },
                                    conflicts: item_conflicts,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(Ll1Result { conflicts })
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::NonTerminal(s) => write!(f, "{}", s),
            Item::Terminal(s) => write!(f, "{}", s),
            Item::Regex(s) => write!(f, "{}", s),
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

impl fmt::Display for Bnf {
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
