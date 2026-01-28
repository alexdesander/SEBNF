use std::collections::HashMap;

use indexmap::IndexMap;

use crate::{bnf, sebnf};

pub fn sebnf_to_bnf(sebnf: &sebnf::Sebnf) -> bnf::Bnf {
    let mut ctx = ConverterContext::new();

    // Convert all rules
    let mut original_rules: Vec<(String, Vec<Vec<bnf::Item>>)> = Vec::new();
    for (name, alternatives) in &sebnf.rules {
        let mut bnf_alternatives = Vec::new();
        for alt in alternatives {
            bnf_alternatives.push(ctx.convert_sequence(alt));
        }
        original_rules.push((name.clone(), bnf_alternatives));
    }

    // Build new index map to preserve correct order
    let mut final_rules = IndexMap::new();
    for (name, alts) in original_rules {
        final_rules.insert(name, alts);
    }
    for (name, alts) in ctx.bnf_rules {
        final_rules.insert(name, alts);
    }

    bnf::Bnf { rules: final_rules }
}

struct ConverterContext {
    bnf_rules: IndexMap<String, Vec<Vec<bnf::Item>>>,
    rule_cache: HashMap<String, String>,
    // Separate cache: repetition bodies contain self-references, can't hash before naming
    rep_cache: HashMap<String, String>,
    uid_counter: usize,
}

impl ConverterContext {
    fn new() -> Self {
        Self {
            bnf_rules: IndexMap::new(),
            rule_cache: HashMap::new(),
            rep_cache: HashMap::new(),
            uid_counter: 0,
        }
    }

    fn next_name(&mut self, prefix: &str) -> String {
        let name = format!("___{}_{}", prefix, self.uid_counter);
        self.uid_counter += 1;
        name
    }

    fn convert_sequence(&mut self, items: &[sebnf::Item]) -> Vec<bnf::Item> {
        items.iter().map(|item| self.convert_item(item)).collect()
    }

    fn convert_item(&mut self, item: &sebnf::Item) -> bnf::Item {
        match item {
            sebnf::Item::NonTerminal(s) => bnf::Item::NonTerminal(s.clone()),
            sebnf::Item::Terminal(s) => bnf::Item::Terminal(s.clone()),
            sebnf::Item::Regex(s) => bnf::Item::Regex(s.clone()),

            // [ A B ] -> __opt_N := A B | epsilon
            sebnf::Item::Optional(children) => {
                let converted_seq = self.convert_sequence(children);
                let body = vec![converted_seq, vec![]];
                let key = format!("{:?}", body);

                let name = if let Some(existing_name) = self.rule_cache.get(&key) {
                    existing_name.clone()
                } else {
                    let new_name = self.next_name("opt");
                    self.rule_cache.insert(key, new_name.clone());
                    self.bnf_rules.insert(new_name.clone(), body);
                    new_name
                };

                bnf::Item::NonTerminal(name)
            }

            // ( A | B ) -> __choice_N := A | B
            sebnf::Item::Choice(alternatives) => {
                let mut body = Vec::new();
                for alt in alternatives {
                    body.push(self.convert_sequence(alt));
                }

                let key = format!("{:?}", body);

                let name = if let Some(existing_name) = self.rule_cache.get(&key) {
                    existing_name.clone()
                } else {
                    let new_name = self.next_name("choice");
                    self.rule_cache.insert(key, new_name.clone());
                    self.bnf_rules.insert(new_name.clone(), body);
                    new_name
                };

                bnf::Item::NonTerminal(name)
            }

            // { A B } -> __rep_N := A B __rep_N | epsilon
            sebnf::Item::AnyAmount(children) => {
                let converted_seq = self.convert_sequence(children);
                let key = format!("{:?}", converted_seq);

                let name = if let Some(existing_name) = self.rep_cache.get(&key) {
                    existing_name.clone()
                } else {
                    let new_name = self.next_name("rep");
                    self.rep_cache.insert(key, new_name.clone());

                    let mut recursive_alt = converted_seq;
                    recursive_alt.push(bnf::Item::NonTerminal(new_name.clone()));

                    let body = vec![recursive_alt, vec![]];
                    self.bnf_rules.insert(new_name.clone(), body);
                    new_name
                };

                bnf::Item::NonTerminal(name)
            }
        }
    }
}
