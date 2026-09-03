// MODE: DEV
// PACKAGE: PROD
use regex::Regex;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    ExactText,
    ExactBytes,
    Wildcard,
    ShellWildcard,
    PathWildcard,
    RegexRust,
    RegexPcre2,
    FuzzyEdit,
    FuzzySubsequence,
    FuzzyToken,
    FuzzyNgram,
    FuzzyPhonetic,
    FuzzySoundex,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SearchError {
    #[error("search mode must be explicitly selected")]
    MissingMode,
    #[error("search mode is unsupported: {0}")]
    Unsupported(String),
    #[error("invalid search expression: {0}")]
    InvalidExpression(String),
}

pub fn parse_mode(value: Option<&str>) -> Result<SearchMode, SearchError> {
    match value.ok_or(SearchError::MissingMode)? {
        "exact_text" => Ok(SearchMode::ExactText),
        "exact_bytes" => Ok(SearchMode::ExactBytes),
        "wildcard" => Ok(SearchMode::Wildcard),
        "shell_wildcard" => Ok(SearchMode::ShellWildcard),
        "path_wildcard" => Ok(SearchMode::PathWildcard),
        "regex_rust" => Ok(SearchMode::RegexRust),
        "regex_pcre2" => Ok(SearchMode::RegexPcre2),
        "fuzzy_edit" => Ok(SearchMode::FuzzyEdit),
        "fuzzy_subsequence" => Ok(SearchMode::FuzzySubsequence),
        "fuzzy_token" => Ok(SearchMode::FuzzyToken),
        "fuzzy_ngram" => Ok(SearchMode::FuzzyNgram),
        "fuzzy_phonetic" => Ok(SearchMode::FuzzyPhonetic),
        "fuzzy_soundex" => Ok(SearchMode::FuzzySoundex),
        other => Err(SearchError::Unsupported(other.to_owned())),
    }
}

pub fn matches(
    mode: SearchMode,
    query: &str,
    haystack: &str,
) -> Result<Vec<(usize, usize)>, SearchError> {
    matches_with_gradient(mode, query, haystack, None)
}

pub fn matches_with_gradient(
    mode: SearchMode,
    query: &str,
    haystack: &str,
    gradient: Option<f64>,
) -> Result<Vec<(usize, usize)>, SearchError> {
    let gradient = gradient.unwrap_or(match mode {
        SearchMode::FuzzyEdit => 1.0 / 3.0,
        SearchMode::FuzzyNgram => 0.5,
        SearchMode::FuzzyPhonetic | SearchMode::FuzzySoundex => 1.0,
        SearchMode::FuzzySubsequence | SearchMode::FuzzyToken => 1.0,
        _ => 0.0,
    });
    if !gradient.is_finite() || !(0.0..=1.0).contains(&gradient) {
        return Err(SearchError::InvalidExpression(
            "gradient must be a finite number from 0.0 through 1.0".into(),
        ));
    }
    match mode {
        SearchMode::ExactText | SearchMode::ExactBytes => Ok(find_literal(query, haystack)),
        SearchMode::Wildcard | SearchMode::ShellWildcard | SearchMode::PathWildcard => {
            let regex = Regex::new(&wildcard_regex(query, mode == SearchMode::ShellWildcard))
                .map_err(|error| SearchError::InvalidExpression(error.to_string()))?;
            Ok(regex
                .find_iter(haystack)
                .map(|found| (found.start(), found.end()))
                .collect())
        }
        SearchMode::RegexRust => Regex::new(query)
            .map(|regex| {
                regex
                    .find_iter(haystack)
                    .map(|found| (found.start(), found.end()))
                    .collect()
            })
            .map_err(|error| SearchError::InvalidExpression(error.to_string())),
        SearchMode::RegexPcre2 => pcre2::bytes::Regex::new(query)
            .map(|regex| {
                regex
                    .find_iter(haystack.as_bytes())
                    .filter_map(Result::ok)
                    .map(|found| (found.start(), found.end()))
                    .collect()
            })
            .map_err(|error| SearchError::InvalidExpression(error.to_string())),
        SearchMode::FuzzyEdit => fuzzy_edit(query, haystack, gradient),
        SearchMode::FuzzySubsequence => Ok(if subsequence_score(query, haystack) >= gradient {
            vec![(0, haystack.len())]
        } else {
            Vec::new()
        }),
        SearchMode::FuzzyToken => Ok(if token_score(query, haystack) >= gradient {
            vec![(0, haystack.len())]
        } else {
            Vec::new()
        }),
        SearchMode::FuzzyNgram => Ok(if ngram_score(query, haystack) >= gradient as f32 {
            vec![(0, haystack.len())]
        } else {
            Vec::new()
        }),
        SearchMode::FuzzyPhonetic => Ok(
            if (phonetic(query) == phonetic(haystack)) as u8 as f64 >= gradient {
                vec![(0, haystack.len())]
            } else {
                Vec::new()
            },
        ),
        SearchMode::FuzzySoundex => Ok(
            if (soundex(query) == soundex(haystack)) as u8 as f64 >= gradient {
                vec![(0, haystack.len())]
            } else {
                Vec::new()
            },
        ),
    }
}

pub fn find_bytes(query: &[u8], haystack: &[u8]) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    haystack
        .windows(query.len())
        .enumerate()
        .filter(|(_, window)| *window == query)
        .map(|(start, _)| (start, start + query.len()))
        .collect()
}

fn find_literal(query: &str, haystack: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut cursor = 0;
    while cursor <= haystack.len() {
        let Some(relative) = haystack[cursor..].find(query) else {
            break;
        };
        let start = cursor + relative;
        result.push((start, start + query.len()));
        cursor = start + query.len();
    }
    result
}

fn wildcard_regex(query: &str, shell: bool) -> String {
    let mut expression = String::from("(?s)");
    for character in query.chars() {
        match character {
            '*' => expression.push_str(".*"),
            '?' => expression.push('.'),
            '\\' if shell => expression.push_str("\\\\"),
            character => expression.push_str(&regex::escape(&character.to_string())),
        }
    }
    expression
}

fn fuzzy_edit(
    query: &str,
    haystack: &str,
    gradient: f64,
) -> Result<Vec<(usize, usize)>, SearchError> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let threshold = (query.chars().count() as f64 * gradient).ceil() as usize;
    let mut matches = Vec::new();
    for (start, _) in haystack.char_indices() {
        let target = (start + query.len() + threshold).min(haystack.len());
        let end = haystack
            .char_indices()
            .map(|(index, _)| index)
            .find(|index| *index >= target)
            .unwrap_or(haystack.len());
        let candidate = &haystack[start..end];
        if levenshtein(query, candidate) <= threshold {
            matches.push((start, end));
        }
    }
    Ok(matches)
}

fn levenshtein(left: &str, right: &str) -> usize {
    let mut row: Vec<usize> = (0..=right.chars().count()).collect();
    for (i, left_char) in left.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, right_char) in right.chars().enumerate() {
            let old = row[j + 1];
            row[j + 1] = if left_char == right_char {
                diagonal
            } else {
                1 + diagonal.min(row[j]).min(old)
            };
            diagonal = old;
        }
    }
    row[right.chars().count()]
}

fn subsequence_score(query: &str, haystack: &str) -> f64 {
    let mut haystack = haystack.chars();
    let matched = query
        .chars()
        .filter(|query_char| haystack.any(|candidate| candidate.eq_ignore_ascii_case(query_char)))
        .count();
    let total = query.chars().count();
    if total == 0 {
        0.0
    } else {
        matched as f64 / total as f64
    }
}

fn token_score(query: &str, haystack: &str) -> f64 {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    if tokens.is_empty() {
        return 0.0;
    }
    let matched = tokens
        .iter()
        .filter(|token| {
            haystack
                .split_whitespace()
                .any(|candidate| candidate.eq_ignore_ascii_case(token))
        })
        .count();
    matched as f64 / tokens.len() as f64
}

fn ngram_score(query: &str, haystack: &str) -> f32 {
    let grams: Vec<&str> = query
        .as_bytes()
        .windows(3)
        .map(|bytes| std::str::from_utf8(bytes).unwrap_or(""))
        .collect();
    if grams.is_empty() {
        return 0.0;
    }
    grams
        .iter()
        .filter(|gram| haystack.contains(**gram))
        .count() as f32
        / grams.len() as f32
}

fn phonetic(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphabetic())
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn soundex(value: &str) -> String {
    let mut chars = value
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .map(|character| character.to_ascii_uppercase());
    let Some(first) = chars.next() else {
        return "0000".into();
    };
    let mut result = String::from(first);
    let mut previous = code(first);
    for character in chars {
        let current = code(character);
        if current != '0' && current != previous {
            result.push(current);
        }
        previous = current;
        if result.len() == 4 {
            break;
        }
    }
    while result.len() < 4 {
        result.push('0');
    }
    result
}

fn code(character: char) -> char {
    match character {
        'B' | 'F' | 'P' | 'V' => '1',
        'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
        'D' | 'T' => '3',
        'L' => '4',
        'M' | 'N' => '5',
        'R' => '6',
        _ => '0',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modes_are_explicit() {
        assert_eq!(parse_mode(None), Err(SearchError::MissingMode));
        assert_eq!(parse_mode(Some("regex_pcre2")), Ok(SearchMode::RegexPcre2));
        assert_eq!(
            parse_mode(Some("path_wildcard")),
            Ok(SearchMode::PathWildcard)
        );
        assert_eq!(
            matches(SearchMode::RegexPcre2, r"cat|dog", "a dog").unwrap(),
            vec![(2, 5)]
        );
    }
    #[test]
    fn literal_and_wildcard_find_ranges() {
        assert_eq!(
            matches(SearchMode::ExactText, "cat", "cat scat").unwrap(),
            vec![(0, 3), (5, 8)]
        );
        assert_eq!(
            matches(SearchMode::Wildcard, "c?t", "cat cut").unwrap(),
            vec![(0, 3), (4, 7)]
        );
    }
    #[test]
    fn fuzzy_strategies_are_distinct_options() {
        assert!(!matches(SearchMode::FuzzySubsequence, "ct", "cat")
            .unwrap()
            .is_empty());
        assert!(!matches(SearchMode::FuzzySoundex, "Robert", "Rupert")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn fuzzy_gradient_is_explicit_and_bounded() {
        assert!(
            matches_with_gradient(SearchMode::FuzzyEdit, "cat", "cut", Some(0.0))
                .unwrap()
                .is_empty()
        );
        assert!(
            !matches_with_gradient(SearchMode::FuzzyEdit, "cat", "cut", Some(0.5))
                .unwrap()
                .is_empty()
        );
        assert!(
            !matches_with_gradient(SearchMode::FuzzyToken, "cat dog", "cat", Some(0.5))
                .unwrap()
                .is_empty()
        );
        assert!(matches_with_gradient(SearchMode::FuzzyNgram, "cat", "cat", Some(1.1)).is_err());
    }
}
