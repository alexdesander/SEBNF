use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::bnf::*;
use crate::regex_intersect::{Error as RegexError, do_regexs_intersect};

#[derive(Debug, Clone)]
pub struct Sets {
    pub first: HashMap<String, HashSet<SetItem>>,
    pub follow: HashMap<String, HashSet<SetItem>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SetItem {
    Terminal(String),
    Regex(String),
    Epsilon,
    EndOfInput,
}

impl From<&Item> for SetItem {
    fn from(item: &Item) -> Self {
        match item {
            Item::Terminal(s) => SetItem::Terminal(s.clone()),
            Item::Regex(s) => SetItem::Regex(s.clone()),
            Item::NonTerminal(_) => panic!("NonTerminal cannot be converted to SetItem"),
        }
    }
}

impl fmt::Display for SetItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetItem::Terminal(s) => {
                let s = strip_terminal_quotes(s);
                write!(f, "{}", s)
            }
            SetItem::Regex(s) => write!(f, "{}", s),
            SetItem::Epsilon => write!(f, "ε"),
            SetItem::EndOfInput => write!(f, "$"),
        }
    }
}

impl fmt::Display for Sets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "FIRST Sets:")?;
        let mut first_keys: Vec<_> = self.first.keys().collect();
        first_keys.sort();
        for key in first_keys {
            let items = &self.first[key];
            let mut sorted_items: Vec<_> = items.iter().map(|i| i.to_string()).collect();
            sorted_items.sort();
            writeln!(f, "  {}:", key)?;
            for item in &sorted_items {
                writeln!(f, "    {}", item)?;
            }
        }

        writeln!(f)?;
        writeln!(f, "FOLLOW Sets:")?;
        let mut follow_keys: Vec<_> = self.follow.keys().collect();
        follow_keys.sort();
        for key in follow_keys {
            let items = &self.follow[key];
            let mut sorted_items: Vec<_> = items.iter().map(|i| i.to_string()).collect();
            sorted_items.sort();
            writeln!(f, "  {}:", key)?;
            for item in &sorted_items {
                writeln!(f, "    {}", item)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Ll1Result {
    pub conflicts: Vec<Ll1Conflict>,
}

impl Ll1Result {
    pub fn is_ll1(&self) -> bool {
        self.conflicts.is_empty()
    }
}

impl fmt::Display for Ll1Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.conflicts.is_empty() {
            writeln!(f, "Grammar is LL(1)")?;
        } else {
            writeln!(
                f,
                "Grammar is NOT LL(1). Found {} conflict(s):",
                self.conflicts.len()
            )?;
            for (i, conflict) in self.conflicts.iter().enumerate() {
                writeln!(f, "\n{}. {}", i + 1, conflict)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Ll1Conflict {
    pub non_terminal: String,
    pub kind: Ll1ConflictKind,
    pub conflicts: Vec<SetItemConflict>,
}

impl fmt::Display for Ll1Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Non-terminal '{}': ", self.non_terminal)?;
        match &self.kind {
            Ll1ConflictKind::FirstFirst {
                production1,
                production2,
            } => {
                writeln!(f, "FIRST/FIRST conflict")?;
                writeln!(f, "   Production 1: {}", format_production(production1))?;
                writeln!(f, "   Production 2: {}", format_production(production2))?;
            }
            Ll1ConflictKind::FirstFollow {
                nullable_production,
                other_production,
            } => {
                writeln!(f, "FIRST/FOLLOW conflict")?;
                writeln!(
                    f,
                    "   Nullable production: {}",
                    format_production(nullable_production)
                )?;
                writeln!(
                    f,
                    "   Other production: {}",
                    format_production(other_production)
                )?;
            }
        }
        writeln!(f, "   Conflicts:")?;
        for conflict in &self.conflicts {
            writeln!(f, "     - {}", conflict)?;
        }
        Ok(())
    }
}

fn format_production(items: &[Item]) -> String {
    if items.is_empty() {
        "ε".to_string()
    } else {
        items
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone)]
pub enum Ll1ConflictKind {
    /// Two productions have overlapping FIRST sets
    FirstFirst {
        production1: Vec<Item>,
        production2: Vec<Item>,
    },
    /// A nullable production's FOLLOW set overlaps with another production's FIRST set
    FirstFollow {
        nullable_production: Vec<Item>,
        other_production: Vec<Item>,
    },
}

/// Returns (FIRST(sequence) without ε, sequence_is_nullable)
pub fn first_of_sequence(
    sequence: &[Item],
    first_sets: &HashMap<String, HashSet<SetItem>>,
) -> (HashSet<SetItem>, bool) {
    let mut firsts = HashSet::new();
    let mut nullable = true;

    for item in sequence {
        match item {
            Item::Terminal(_) | Item::Regex(_) => {
                firsts.insert(SetItem::from(item));
                nullable = false;
                break;
            }
            Item::NonTerminal(nt) => {
                let nt_firsts = first_sets.get(nt).cloned().unwrap_or_default();
                let has_epsilon = nt_firsts.contains(&SetItem::Epsilon);
                firsts.extend(
                    nt_firsts
                        .into_iter()
                        .filter(|f| !matches!(f, SetItem::Epsilon)),
                );
                if !has_epsilon {
                    nullable = false;
                    break;
                }
            }
        }
    }

    (firsts, nullable)
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum Ll1Error {
    #[error("invalid regex pattern in grammar: {pattern}")]
    InvalidRegex {
        pattern: String,
        #[source]
        source: RegexError,
    },
}

fn strip_regex_delimiters(s: &str) -> &str {
    s.strip_prefix('/')
        .unwrap_or(s)
        .strip_suffix('/')
        .unwrap_or(s)
}

fn strip_terminal_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .unwrap_or(s)
        .strip_suffix('"')
        .unwrap_or(s)
}

#[derive(Debug, Clone)]
pub struct SetItemConflict {
    pub item1: SetItem,
    pub item2: SetItem,
    pub witness: Option<String>,
}

impl fmt::Display for SetItemConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ∩ {}", self.item1, self.item2)?;
        if let Some(ref w) = self.witness {
            write!(f, " (e.g., \"{}\")", w)?;
        }
        Ok(())
    }
}

pub fn find_set_conflicts(
    set1: &HashSet<SetItem>,
    set2: &HashSet<SetItem>,
) -> Result<Vec<SetItemConflict>, Ll1Error> {
    let mut conflicts = Vec::new();

    for item1 in set1 {
        for item2 in set2 {
            if let Some(conflict) = check_item_conflict(item1, item2)? {
                conflicts.push(conflict);
            }
        }
    }

    Ok(conflicts)
}

fn check_item_conflict(
    item1: &SetItem,
    item2: &SetItem,
) -> Result<Option<SetItemConflict>, Ll1Error> {
    match (item1, item2) {
        (SetItem::Terminal(t1), SetItem::Terminal(t2)) => {
            let s1 = strip_terminal_quotes(t1);
            let s2 = strip_terminal_quotes(t2);
            if s1 == s2 {
                Ok(Some(SetItemConflict {
                    item1: item1.clone(),
                    item2: item2.clone(),
                    witness: Some(s1.to_string()),
                }))
            } else {
                Ok(None)
            }
        }

        (SetItem::Regex(r1), SetItem::Regex(r2)) => {
            let p1 = strip_regex_delimiters(r1);
            let p2 = strip_regex_delimiters(r2);
            match do_regexs_intersect(p1, p2) {
                Ok(Some(witness)) => Ok(Some(SetItemConflict {
                    item1: item1.clone(),
                    item2: item2.clone(),
                    witness: Some(witness),
                })),
                Ok(None) => Ok(None),
                Err(e) => Err(Ll1Error::InvalidRegex {
                    pattern: format!("{} or {}", r1, r2),
                    source: e,
                }),
            }
        }

        (SetItem::Regex(r), SetItem::Terminal(t)) | (SetItem::Terminal(t), SetItem::Regex(r)) => {
            let pattern = strip_regex_delimiters(r);
            let terminal = strip_terminal_quotes(t);
            let escaped_terminal = regex_syntax::escape(terminal);
            match do_regexs_intersect(pattern, &escaped_terminal) {
                Ok(Some(witness)) => Ok(Some(SetItemConflict {
                    item1: item1.clone(),
                    item2: item2.clone(),
                    witness: Some(witness),
                })),
                Ok(None) => Ok(None),
                Err(e) => Err(Ll1Error::InvalidRegex {
                    pattern: r.to_string(),
                    source: e,
                }),
            }
        }

        (SetItem::Epsilon, SetItem::Epsilon) => Ok(Some(SetItemConflict {
            item1: item1.clone(),
            item2: item2.clone(),
            witness: None,
        })),

        (SetItem::EndOfInput, SetItem::EndOfInput) => Ok(Some(SetItemConflict {
            item1: item1.clone(),
            item2: item2.clone(),
            witness: None,
        })),

        _ => Ok(None),
    }
}

pub fn extract_sets(bnf: &Bnf) -> Sets {
    let mut first_sets: HashMap<String, HashSet<SetItem>> = HashMap::new();
    let mut follow_sets: HashMap<String, HashSet<SetItem>> = HashMap::new();

    for nt in bnf.rules.keys() {
        first_sets.insert(nt.clone(), HashSet::new());
        follow_sets.insert(nt.clone(), HashSet::new());
    }

    // FIRST sets: fixed-point iteration until no changes
    let mut changed = true;
    while changed {
        changed = false;

        for (lhs, productions) in &bnf.rules {
            for production in productions {
                let (firsts, nullable) = first_of_sequence(production, &first_sets);

                let lhs_set = first_sets.get_mut(lhs).unwrap();
                for f in firsts {
                    changed |= lhs_set.insert(f);
                }
                if nullable {
                    changed |= lhs_set.insert(SetItem::Epsilon);
                }
            }
        }
    }

    // FOLLOW sets: start symbol gets $
    if let Some((start_symbol, _)) = bnf.rules.first() {
        follow_sets
            .get_mut(start_symbol)
            .unwrap()
            .insert(SetItem::EndOfInput);
    }

    // FOLLOW sets: fixed-point iteration
    // For A -> αBβ: FOLLOW(B) ∪= FIRST(β)\{ε}; if β ⇒* ε then FOLLOW(B) ∪= FOLLOW(A)
    changed = true;
    while changed {
        changed = false;

        for (lhs, productions) in &bnf.rules {
            for production in productions {
                for i in 0..production.len() {
                    let Item::NonTerminal(current_nt) = &production[i] else {
                        continue;
                    };
                    let Some(current_follow) = follow_sets.get_mut(current_nt) else {
                        continue;
                    };

                    let beta = &production[i + 1..];
                    let (beta_firsts, beta_nullable) = first_of_sequence(beta, &first_sets);

                    for f in beta_firsts {
                        changed |= current_follow.insert(f);
                    }

                    if beta_nullable {
                        let lhs_follows = follow_sets.get(lhs).cloned().unwrap_or_default();
                        let current_follow = follow_sets.get_mut(current_nt).unwrap();
                        for f in lhs_follows {
                            changed |= current_follow.insert(f);
                        }
                    }
                }
            }
        }
    }

    Sets {
        first: first_sets,
        follow: follow_sets,
    }
}
