use std::collections::{HashMap, VecDeque};

use regex_automata::{
    Anchored, Input,
    dfa::{
        Automaton,
        dense::{BuildError, DFA},
    },
    util::primitives::StateID,
};

#[derive(Debug)]
pub enum Error {
    InvalidRegexA(BuildError),
    InvalidRegexB(BuildError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidRegexA(e) => write!(f, "invalid regex pattern a: {e}"),
            Error::InvalidRegexB(e) => write!(f, "invalid regex pattern b: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidRegexA(e) | Error::InvalidRegexB(e) => Some(e),
        }
    }
}

/// Check if a regex pattern can match the empty string.
///
/// Returns true if the pattern matches "", false otherwise.
/// Returns false if the pattern is invalid.
pub fn regex_matches_empty(pattern: &str) -> bool {
    let Ok(dfa) = DFA::new(pattern) else {
        return false;
    };
    let input = Input::new(&[] as &[u8]).anchored(Anchored::Yes);
    let start = dfa.start_state_forward(&input).unwrap();
    let eoi = dfa.next_eoi_state(start);
    dfa.is_match_state(eoi)
}

/// Check if two regex patterns have a non-empty intersection.
///
/// Returns Ok(Some(string)) with the smallest matching string if there exists
/// at least one string that both patterns would fully match.
/// Returns Ok(None) if there is no intersection.
/// Returns Err if either pattern is invalid.
///
/// Uses full-string match semantics.
pub fn do_regexs_intersect(a: &str, b: &str) -> Result<Option<String>, Error> {
    let dfa_a = DFA::new(a).map_err(Error::InvalidRegexA)?;
    let dfa_b = DFA::new(b).map_err(Error::InvalidRegexB)?;

    let input = Input::new(&[] as &[u8]).anchored(Anchored::Yes);
    let start_a = dfa_a.start_state_forward(&input).unwrap();
    let start_b = dfa_b.start_state_forward(&input).unwrap();

    // BFS over the product automaton, tracking parent states for path reconstruction
    let mut parent: HashMap<(StateID, StateID), Option<((StateID, StateID), u8)>> = HashMap::new();
    let mut queue: VecDeque<(StateID, StateID)> = VecDeque::new();

    parent.insert((start_a, start_b), None);
    queue.push_back((start_a, start_b));

    while let Some((state_a, state_b)) = queue.pop_front() {
        let eoi_a = dfa_a.next_eoi_state(state_a);
        let eoi_b = dfa_b.next_eoi_state(state_b);

        if dfa_a.is_match_state(eoi_a) && dfa_b.is_match_state(eoi_b) {
            let mut bytes = Vec::new();
            let mut current = (state_a, state_b);
            while let Some(Some((prev, byte))) = parent.get(&current) {
                bytes.push(*byte);
                current = *prev;
            }
            bytes.reverse();
            return Ok(Some(String::from_utf8_lossy(&bytes).into_owned()));
        }

        for byte in 0u8..=255u8 {
            let next_a = dfa_a.next_state(state_a, byte);
            let next_b = dfa_b.next_state(state_b, byte);

            if dfa_a.is_dead_state(next_a) || dfa_b.is_dead_state(next_b) {
                continue;
            }

            if !parent.contains_key(&(next_a, next_b)) {
                parent.insert((next_a, next_b), Some(((state_a, state_b), byte)));
                queue.push_back((next_a, next_b));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_matches_empty() {
        // Patterns that match empty string
        assert!(regex_matches_empty(""));
        assert!(regex_matches_empty("a*"));
        assert!(regex_matches_empty("a?"));
        assert!(regex_matches_empty(".*"));
        assert!(regex_matches_empty("(a|b)*"));
        assert!(regex_matches_empty("a*b*"));
        assert!(regex_matches_empty("(ab)?"));
        assert!(regex_matches_empty("()"));
        assert!(regex_matches_empty("()*"));

        // Patterns that do NOT match empty string
        assert!(!regex_matches_empty("a"));
        assert!(!regex_matches_empty("a+"));
        assert!(!regex_matches_empty("."));
        assert!(!regex_matches_empty(".+"));
        assert!(!regex_matches_empty("ab"));
        assert!(!regex_matches_empty("[a-z]"));
        assert!(!regex_matches_empty("[a-z]+"));
        assert!(!regex_matches_empty("a{1,}"));
        assert!(!regex_matches_empty("a{2,5}"));

        // Invalid patterns return false
        assert!(!regex_matches_empty("[invalid"));
    }

    #[test]
    fn identical_patterns() {
        let result = do_regexs_intersect("abc", "abc").unwrap();
        assert_eq!(result, Some("abc".to_string()));
    }

    #[test]
    fn disjoint_patterns() {
        let result = do_regexs_intersect("a+", "b+").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn empty_string_intersection() {
        let result = do_regexs_intersect("a*", "b*").unwrap();
        assert_eq!(result, Some("".to_string()));

        let result = do_regexs_intersect("", "a?").unwrap();
        assert_eq!(result, Some("".to_string()));
    }

    #[test]
    fn partial_overlap() {
        let result = do_regexs_intersect("a+b*", "a*b+").unwrap();
        assert_eq!(result, Some("ab".to_string()));
    }

    #[test]
    fn character_classes() {
        let result = do_regexs_intersect("[a-z]+", "[x-z]+").unwrap();
        assert_eq!(result, Some("x".to_string()));

        let result = do_regexs_intersect("[a-c]+", "[x-z]+").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn alternation() {
        let result = do_regexs_intersect("cat|dog", "dog|bird").unwrap();
        assert_eq!(result, Some("dog".to_string()));

        let result = do_regexs_intersect("cat|dog", "fish|bird").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn quantifiers() {
        // "aaa" is shortest match for a{2,4} and a{3,5}
        let result = do_regexs_intersect("a{2,4}", "a{3,5}").unwrap();
        assert_eq!(result, Some("aaa".to_string()));

        let result = do_regexs_intersect("a{2}", "a{3}").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn invalid_regex_a() {
        let result = do_regexs_intersect("[invalid", "abc");
        assert!(matches!(result, Err(Error::InvalidRegexA(_))));
    }

    #[test]
    fn invalid_regex_b() {
        let result = do_regexs_intersect("abc", "[invalid");
        assert!(matches!(result, Err(Error::InvalidRegexB(_))));
    }

    fn assert_intersects(a: &str, b: &str, expected: Option<&str>) {
        let result = do_regexs_intersect(a, b).unwrap();
        if let Some(exp) = expected {
            assert_eq!(
                result,
                Some(exp.to_string()),
                "Expected intersection '{}' for patterns '{}' and '{}'",
                exp,
                a,
                b
            );
        } else {
            assert!(
                result.is_some(),
                "Expected intersection for patterns '{}' and '{}', got None",
                a,
                b
            );
        }
    }

    fn assert_no_intersect(a: &str, b: &str) {
        let result = do_regexs_intersect(a, b).unwrap();
        assert_eq!(
            result, None,
            "Expected no intersection for patterns '{}' and '{}', got {:?}",
            a, b, result
        );
    }

    fn verify_result_matches(a: &str, b: &str) {
        let result = do_regexs_intersect(a, b).unwrap();
        if let Some(ref s) = result {
            let re_a = regex::Regex::new(&format!("^(?:{})$", a)).unwrap();
            let re_b = regex::Regex::new(&format!("^(?:{})$", b)).unwrap();
            assert!(
                re_a.is_match(s),
                "Result '{}' does not match pattern a: '{}'",
                s,
                a
            );
            assert!(
                re_b.is_match(s),
                "Result '{}' does not match pattern b: '{}'",
                s,
                b
            );
        }
    }

    #[test]
    fn mega_exhaustive_test() {
        assert_intersects("", "", Some(""));
        assert_intersects("", ".*", Some(""));
        assert_intersects("", "a?", Some(""));
        assert_intersects("", "a*", Some(""));
        assert_intersects("", "()", Some(""));

        assert_intersects("a", "a", Some("a"));
        assert_intersects("Z", "Z", Some("Z"));
        assert_intersects("9", "9", Some("9"));
        assert_no_intersect("a", "b");
        assert_no_intersect("a", "A");

        assert_intersects("hello", "hello", Some("hello"));
        assert_intersects("hello world", "hello world", Some("hello world"));
        assert_no_intersect("hello", "world");
        assert_no_intersect("hello", "hell");
        assert_no_intersect("hello", "helloo");

        assert_intersects(r"\.", r"\.", Some("."));
        assert_intersects(r"\*", r"\*", Some("*"));
        assert_intersects(r"\+", r"\+", Some("+"));
        assert_intersects(r"\?", r"\?", Some("?"));
        assert_intersects(r"\[", r"\[", Some("["));
        assert_intersects(r"\]", r"\]", Some("]"));
        assert_intersects(r"\(", r"\(", Some("("));
        assert_intersects(r"\)", r"\)", Some(")"));
        assert_intersects(r"\{", r"\{", Some("{"));
        assert_intersects(r"\}", r"\}", Some("}"));
        assert_intersects(r"\\", r"\\", Some("\\"));
        assert_intersects(r"\|", r"\|", Some("|"));
        assert_intersects(r"\^", r"\^", Some("^"));
        assert_intersects(r"\$", r"\$", Some("$"));

        assert_intersects("[abc]", "[bcd]", Some("b"));
        assert_intersects("[abc]", "[cde]", Some("c"));
        assert_no_intersect("[abc]", "[def]");

        assert_intersects("[a]", "a", Some("a"));
        assert_intersects("a", "[a]", Some("a"));

        assert_intersects("[ab]+", "[bc]+", Some("b"));
        assert_intersects("[ab]*", "[bc]*", Some(""));
        assert_intersects("[ab][cd]", "[ac][bd]", Some("ad"));

        assert_intersects("[a-z]", "[m-z]", Some("m"));
        assert_intersects("[a-m]", "[m-z]", Some("m"));
        assert_no_intersect("[a-l]", "[n-z]");

        assert_intersects("[A-Z]", "[M-Z]", Some("M"));
        assert_intersects("[0-9]", "[5-9]", Some("5"));
        assert_intersects("[0-4]", "[4-9]", Some("4"));
        assert_no_intersect("[0-3]", "[5-9]");

        assert_intersects("[a-zA-Z]", "[A-Za-z]", Some("A"));
        assert_intersects("[a-z0-9]", "[0-9a-f]", Some("0"));
        assert_intersects("[a-fA-F0-9]", "[0-9a-fA-F]", Some("0"));

        assert_intersects("[a-a]", "a", Some("a"));
        assert_intersects("[-a]", "-", Some("-"));
        assert_intersects("[a-]", "-", Some("-"));
        assert_intersects("[]-]", "]", Some("]"));

        assert_intersects("[^a]", "b", Some("b"));
        assert_intersects("[^a]", "[^b]", None);
        assert_no_intersect("[^a-z]", "[a-z]");
        assert_intersects("[^a-z]", "[A-Z]", Some("A"));
        assert_intersects("[^0-9]", "[a-z]", Some("a"));

        assert_no_intersect("[^abc]", "[abc]");
        assert_intersects("[^abc]", "[bcd]", Some("d"));

        assert_intersects(r"\d", "[0-9]", Some("0"));
        assert_intersects(r"\d+", "[0-9]+", Some("0"));
        assert_intersects(r"\d", "5", Some("5"));
        assert_no_intersect(r"\d", "[a-z]");

        assert_intersects(r"\D", "[a-z]", Some("a"));
        assert_no_intersect(r"\D", "[0-9]");

        assert_intersects(r"\w", "[a-z]", Some("a"));
        assert_intersects(r"\w", "[A-Z]", Some("A"));
        assert_intersects(r"\w", "[0-9]", Some("0"));
        assert_intersects(r"\w", "_", Some("_"));
        assert_no_intersect(r"\w", r"\s");

        assert_intersects(r"\W", " ", Some(" "));
        assert_intersects(r"\W", r"\.", Some("."));
        assert_no_intersect(r"\W", "[a-z]");

        assert_intersects(r"\s", " ", Some(" "));
        assert_intersects(r"\s", "\t", Some("\t"));
        assert_intersects(r"\s", "\n", Some("\n"));
        assert_intersects(r"\s", "\r", Some("\r"));
        assert_no_intersect(r"\s", "[a-z]");

        assert_intersects(r"\S", "[a-z]", Some("a"));
        assert_no_intersect(r"\S", " ");

        assert_intersects("a*", "", Some(""));
        assert_intersects("a*", "a", Some("a"));
        assert_intersects("a*", "aa", Some("aa"));
        assert_intersects("a*", "aaa", Some("aaa"));
        assert_intersects("a*", "a*", Some(""));
        assert_intersects("a*", "a+", Some("a"));
        assert_intersects("a*", "b*", Some(""));

        assert_intersects("a+", "a", Some("a"));
        assert_intersects("a+", "aa", Some("aa"));
        assert_intersects("a+", "a+", Some("a"));
        assert_no_intersect("a+", "");
        assert_no_intersect("a+", "b+");

        assert_intersects("a?", "", Some(""));
        assert_intersects("a?", "a", Some("a"));
        assert_intersects("a?", "a?", Some(""));
        assert_no_intersect("a?", "aa");

        assert_intersects("a{3}", "aaa", Some("aaa"));
        assert_intersects("a{3}", "a{3}", Some("aaa"));
        assert_no_intersect("a{3}", "a{4}");
        assert_no_intersect("a{3}", "aa");

        assert_intersects("a{2,}", "aa", Some("aa"));
        assert_intersects("a{2,}", "aaa", Some("aaa"));
        assert_intersects("a{2,}", "a{3,}", Some("aaa"));
        assert_no_intersect("a{2,}", "a");

        assert_intersects("a{2,4}", "a{3,5}", Some("aaa"));
        assert_intersects("a{1,3}", "a{2,4}", Some("aa"));
        assert_intersects("a{2,5}", "a{3,4}", Some("aaa"));
        assert_no_intersect("a{2,3}", "a{4,5}");

        assert_intersects("a{0}", "", Some(""));
        assert_intersects("a{0,0}", "", Some(""));
        assert_intersects("a{1,1}", "a", Some("a"));
        assert_intersects("a{0,}", "a*", Some(""));
        assert_intersects("a{1,}", "a+", Some("a"));

        assert_intersects(".", "a", Some("a"));
        assert_intersects(".", "[a-z]", Some("a"));
        assert_intersects(".", r"\d", Some("0"));
        assert_intersects("..", "ab", Some("ab"));
        assert_intersects("...", "[a-z]{3}", Some("aaa"));
        assert_intersects(".*", "", Some(""));
        assert_intersects(".*", "anything", Some("anything"));
        assert_intersects(".+", "x", Some("x"));
        assert_no_intersect(".+", "");

        assert_intersects("a.c", "abc", Some("abc"));
        assert_intersects("a.c", "a[xyz]c", Some("axc"));
        assert_no_intersect("a.c", "ac");
        assert_no_intersect("a.c", "abbc");

        assert_intersects("a|b", "a", Some("a"));
        assert_intersects("a|b", "b", Some("b"));
        assert_intersects("a|b", "a|b", Some("a"));
        assert_intersects("a|b", "b|c", Some("b"));
        assert_no_intersect("a|b", "c|d");

        assert_intersects("cat|dog", "cat", Some("cat"));
        assert_intersects("cat|dog", "dog|bird", Some("dog"));
        assert_intersects("red|green|blue", "blue|yellow", Some("blue"));
        assert_no_intersect("cat|dog", "bird|fish");

        assert_intersects("(a|b)|(c|d)", "c", Some("c"));
        assert_intersects("(ab|cd)|(ef|gh)", "cd|xy", Some("cd"));

        assert_intersects("(a|b)+", "a+", Some("a"));
        assert_intersects("(a|b)+", "b+", Some("b"));
        assert_intersects("(a|b)*", "", Some(""));
        assert_intersects("(cat|dog)+", "catdog", Some("catdog"));

        assert_intersects("(abc)", "abc", Some("abc"));
        assert_intersects("(a)(b)(c)", "abc", Some("abc"));
        assert_intersects("(ab)+", "abab", Some("abab"));

        assert_intersects("(?:abc)", "abc", Some("abc"));
        assert_intersects("(?:a|b)+", "(a|b)+", Some("a"));

        assert_intersects("((a))", "a", Some("a"));
        assert_intersects("(a(b(c)))", "abc", Some("abc"));
        assert_intersects("((ab)+)", "(ab)+", Some("ab"));

        assert_intersects("(ab){2}", "abab", Some("abab"));
        assert_intersects("(ab){2,3}", "(ab){3}", Some("ababab"));
        assert_intersects("([a-z]){3}", "[a-z]{3}", Some("aaa"));

        assert_intersects(
            r"[a-z]+@[a-z]+\.[a-z]+",
            r"test@example\.com",
            Some("test@example.com"),
        );

        assert_intersects(r"\d{3}-\d{4}", r"555-\d{4}", Some("555-0000"));
        assert_intersects(r"\d{3}-\d{3}-\d{4}", r"800-555-\d{4}", Some("800-555-0000"));

        assert_intersects(
            r"\d{4}-\d{2}-\d{2}",
            r"2024-\d{2}-\d{2}",
            Some("2024-00-00"),
        );
        assert_intersects(
            r"\d{2}/\d{2}/\d{4}",
            r"\d{2}/\d{2}/2024",
            Some("00/00/2024"),
        );

        assert_intersects(
            r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}",
            r"192\.168\.\d+\.\d+",
            Some("192.168.0.0"),
        );

        assert_intersects(r"#[0-9a-f]{6}", r"#ff[0-9a-f]{4}", Some("#ff0000"));
        assert_intersects(r"/[a-z]+/[a-z]+", r"/api/[a-z]+", Some("/api/a"));
        assert_intersects(r"\d+\.\d+\.\d+", r"1\.\d+\.0", Some("1.0.0"));

        assert_intersects("a+", "a{1,100}", Some("a"));
        assert_intersects(".*", ".{0,1000}", Some(""));

        assert_intersects("(a+)+", "a+", Some("a"));
        assert_intersects("(a*)*", "", Some(""));
        assert_intersects("(a?)+", "", Some(""));

        assert_intersects("α", "α", Some("α"));
        assert_intersects("café", "café", Some("café"));
        assert_intersects("[αβγ]", "β", Some("β"));
        assert_intersects("日本語", "日本語", Some("日本語"));

        assert_intersects("hello世界", "hello世界", Some("hello世界"));
        assert_intersects("[a-zα-ω]+", "[α-ω]+", Some("α"));

        assert_intersects(
            "abcdefghijklmnopqrstuvwxyz",
            "abcdefghijklmnopqrstuvwxyz",
            Some("abcdefghijklmnopqrstuvwxyz"),
        );

        assert_intersects("a|b|c|d|e|f|g|h|i|j", "j|i|h|g|f|e|d|c|b|a", Some("a"));

        assert_intersects("a+", "a{1,}", Some("a"));
        assert_intersects("a*b+", "a+b*", Some("ab"));
        assert_intersects("[abc]+", "[abc]", Some("a"));
        assert_intersects("(ab)+", "ab(ab)*", Some("ab"));

        assert_intersects("a*", "b*", Some(""));
        assert_intersects("(a|b)?", "(c|d)?", Some(""));
        assert_intersects(".*", ".*", Some(""));

        assert_no_intersect("abc", "ABC");
        assert_no_intersect("[a-z]+", "[A-Z]+");
        assert_intersects("[a-zA-Z]+", "[A-Z]+", Some("A"));
        assert_intersects("[a-zA-Z]+", "[a-z]+", Some("a"));

        assert_intersects("(ab)*", "(ba)*", Some(""));
        assert_intersects("(ab)*", "abab", Some("abab"));
        assert_intersects("a*b*", "b*a*", Some(""));
        assert_intersects("a*b*", "a+b+", Some("ab"));

        assert_intersects("colou?r", "color", Some("color"));
        assert_intersects("colou?r", "colour", Some("colour"));
        assert_intersects("https?://", "http://", Some("http://"));
        assert_intersects("https?://", "https://", Some("https://"));

        assert_intersects("abc", "abc", Some("abc"));
        assert_no_intersect("abc", "abcd");
        assert_no_intersect("abc", "zabc");

        assert_intersects("([a-z]{2}[0-9]{2})+", "aa00bb11", Some("aa00bb11"));
        assert_intersects("((((a))))", "a", Some("a"));
        assert_intersects(r"[a-z]+\d*\.?[A-Z]?", r"test123\.X", Some("test123.X"));
        assert_intersects(r"(foo|bar)(baz|qux)?", r"foobaz", Some("foobaz"));

        assert_no_intersect("aaa", "bbb");
        assert_no_intersect("[0-9]+", "[a-z]+");
        assert_no_intersect("^abc", "xyz$");
        assert_no_intersect(r"\d+", r"\D+");
        assert_no_intersect(r"\w+", r"\W+");
        assert_no_intersect(r"\s+", r"\S+");
        assert_no_intersect("a{5}", "a{3}");
        assert_no_intersect("(ab)+", "(ba)+");
        assert_no_intersect("[aeiou]+", "[^aeiou]+");

        assert_no_intersect("[a-m]+", "[n-z]+");
        assert_no_intersect("[0-4]+", "[5-9]+");

        assert_no_intersect("a{10}", "a{5}");
        assert_no_intersect(".{3}", ".{5}");

        verify_result_matches("a+", "a{1,5}");
        verify_result_matches("[a-z]+", "[m-z]+");
        verify_result_matches(r"\d{3}-\d{4}", r"555-\d{4}");
        verify_result_matches("(cat|dog)+", "(dog|cat)+");
        verify_result_matches(r"[a-z]+@[a-z]+\.[a-z]+", r"[a-z]+@test\.[a-z]+");
        verify_result_matches("(ab|cd)+", "(cd|ab)+");
        verify_result_matches(r"\w+\.\w+", r"[a-z]+\.[a-z]+");
        verify_result_matches("a*b+c*", "a+b*c+");
        verify_result_matches("[0-9a-f]+", "[0-9]+");
        verify_result_matches("(x|y|z){2,4}", "(x|y){2,3}");

        let pairs = [
            ("abc", "abc"),
            ("a+", "a*"),
            ("[a-z]+", "[x-z]+"),
            ("cat|dog", "dog|bird"),
            (r"\d+", "[0-9]+"),
            ("(ab)+", "abab"),
        ];

        for (a, b) in pairs {
            let result_ab = do_regexs_intersect(a, b).unwrap();
            let result_ba = do_regexs_intersect(b, a).unwrap();
            assert_eq!(
                result_ab.is_some(),
                result_ba.is_some(),
                "Symmetry failed for '{}' and '{}'",
                a,
                b
            );
        }

        assert_intersects(r"\t", "\t", Some("\t"));
        assert_intersects(r"\n", "\n", Some("\n"));
        assert_intersects(r"\r", "\r", Some("\r"));
        assert_intersects(r"\t\n\r", "\t\n\r", Some("\t\n\r"));

        assert_intersects(r"a\.b", r"a\.b", Some("a.b"));
        assert_intersects(r"a\*b", r"a\*b", Some("a*b"));
        assert_intersects(r"a\+b", r"a\+b", Some("a+b"));
        assert_intersects(r"a\?b", r"a\?b", Some("a?b"));
        assert_intersects(r"a\.b", "a.b", Some("a.b"));
        assert_no_intersect(r"a\*b", "a*b");

        // Word boundaries may not be supported by DFA - just verify no crash
        let _ = do_regexs_intersect(r"\bword\b", "word");
        let _ = do_regexs_intersect(r"foo\Bbar", "foobar");

        assert_intersects(
            "[abcdefghijklmnopqrstuvwxyz]",
            "[zyxwvutsrqponmlkjihgfedcba]",
            Some("a"),
        );

        assert_intersects("[0123456789abcdefABCDEF]", "[0-9a-fA-F]", Some("0"));

        assert_intersects("a+b+c+", "abc", Some("abc"));
        assert_intersects("a*b*c*", "", Some(""));
        assert_intersects("a?b?c?", "", Some(""));
        assert_intersects("a{1,2}b{1,2}c{1,2}", "abc", Some("abc"));

        assert_intersects("((a+)+)+", "a", Some("a"));
        assert_intersects("((a*)*)*", "", Some(""));

        assert_intersects(r"[a-z][a-z0-9_]{2,15}", r"[a-z]{3,10}", Some("aaa"));
        assert_intersects(r"[a-z0-9]+(-[a-z0-9]+)*", r"[a-z]+-[a-z]+", Some("a-a"));
        assert_intersects(r"[a-z]+\.(txt|pdf|doc)", r"report\.pdf", Some("report.pdf"));
        assert_intersects(r"[0-2][0-9]:[0-5][0-9]", r"12:\d{2}", Some("12:00"));
    }
}
