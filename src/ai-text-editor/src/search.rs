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
        SearchMode::FuzzySubsequence => Ok(subsequence_ranges(query, haystack, gradient)),
        SearchMode::FuzzyToken => Ok(token_ranges(query, haystack, gradient)),
        SearchMode::FuzzyNgram => Ok(ngram_ranges(query, haystack, gradient as f32)),
        SearchMode::FuzzyPhonetic => Ok(phonetic_ranges(query, haystack, gradient, false)),
        SearchMode::FuzzySoundex => Ok(phonetic_ranges(query, haystack, gradient, true)),
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
    let haystack_indices: Vec<usize> = haystack
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(haystack.len()))
        .collect();
    let query_len = query.chars().count();
    let mut matches = Vec::new();
    for (start_char, &start) in haystack_indices
        .iter()
        .enumerate()
        .take(haystack_indices.len() - 1)
    {
        let minimum_len = query_len.saturating_sub(threshold);
        let maximum_len = query_len.saturating_add(threshold);
        for candidate_len in minimum_len..=maximum_len {
            let end_char = start_char.saturating_add(candidate_len);
            if end_char >= haystack_indices.len() {
                continue;
            }
            let end = haystack_indices[end_char];
            let candidate = &haystack[start..end];
            if levenshtein(query, candidate) <= threshold {
                matches.push((start, end));
                break;
            }
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

fn subsequence_ranges(query: &str, haystack: &str, gradient: f64) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let query: Vec<char> = query.chars().collect();
    let chars: Vec<(usize, char)> = haystack.char_indices().collect();
    let mut ranges = Vec::new();
    for start in 0..chars.len() {
        let mut query_index = 0;
        let mut end = start;
        while end < chars.len() && query_index < query.len() {
            if chars[end].1.eq_ignore_ascii_case(&query[query_index]) {
                query_index += 1;
            }
            end += 1;
        }
        if query_index as f64 / query.len() as f64 >= gradient {
            let end_byte = chars
                .get(end.saturating_sub(1))
                .map_or(haystack.len(), |(_, ch)| {
                    chars[end.saturating_sub(1)].0 + ch.len_utf8()
                });
            ranges.push((chars[start].0, end_byte));
        }
    }
    ranges
}

fn token_ranges(query: &str, haystack: &str, gradient: f64) -> Vec<(usize, usize)> {
    let wanted: Vec<&str> = query.split_whitespace().collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut cursor = 0;
    let mut tokens = Vec::new();
    for token in haystack.split_whitespace() {
        let Some(relative) = haystack[cursor..].find(token) else {
            continue;
        };
        let start = cursor + relative;
        tokens.push((start, start + token.len(), token));
        cursor = start + token.len();
    }
    let mut ranges = Vec::new();
    for start in 0..tokens.len() {
        let end = (start + wanted.len()).min(tokens.len());
        let matched = wanted[..end - start]
            .iter()
            .zip(tokens[start..end].iter())
            .filter(|(wanted, (_, _, candidate))| wanted.eq_ignore_ascii_case(candidate))
            .count();
        if matched as f64 / wanted.len() as f64 >= gradient {
            ranges.push((tokens[start].0, tokens[end - 1].1));
        }
    }
    ranges
}

fn ngram_ranges(query: &str, haystack: &str, gradient: f32) -> Vec<(usize, usize)> {
    let grams: Vec<&str> = query
        .as_bytes()
        .windows(3)
        .map(|bytes| std::str::from_utf8(bytes).unwrap_or(""))
        .collect();
    if grams.is_empty() {
        return Vec::new();
    }
    let found: Vec<(usize, usize)> = grams
        .iter()
        .filter_map(|gram| {
            let start = haystack.find(gram)?;
            Some((start, start + gram.len()))
        })
        .collect();
    let score = found.len() as f32 / grams.len() as f32;
    if score >= gradient {
        let start = found.iter().map(|(start, _)| *start).min().unwrap();
        let end = found.iter().map(|(_, end)| *end).max().unwrap();
        vec![(start, end)]
    } else {
        Vec::new()
    }
}

fn phonetic_ranges(
    query: &str,
    haystack: &str,
    gradient: f64,
    use_soundex: bool,
) -> Vec<(usize, usize)> {
    let wanted = if use_soundex {
        soundex(query)
    } else {
        phonetic(query)
    };
    if wanted.is_empty() {
        return Vec::new();
    }
    haystack
        .split_whitespace()
        .filter_map(|word| {
            let candidate = if use_soundex {
                soundex(word)
            } else {
                phonetic(word)
            };
            if (candidate == wanted) as u8 as f64 >= gradient {
                let start = haystack.find(word)?;
                Some((start, start + word.len()))
            } else {
                None
            }
        })
        .collect()
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
        assert_eq!(
            matches(SearchMode::FuzzySubsequence, "ct", "cat").unwrap(),
            vec![(0, 3)]
        );
        assert_eq!(
            matches(SearchMode::FuzzyToken, "cat dog", "cat dog here").unwrap(),
            vec![(0, 7)]
        );
        assert_eq!(
            matches(SearchMode::FuzzyNgram, "cat", "before cat after").unwrap(),
            vec![(7, 10)]
        );
        assert_eq!(
            matches(SearchMode::FuzzyPhonetic, "Robert", "Alice Robert").unwrap(),
            vec![(6, 12)]
        );
        assert_eq!(
            matches(SearchMode::FuzzySoundex, "Robert", "Alice Rupert").unwrap(),
            vec![(6, 12)]
        );
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

    #[test]
    fn edit_distance_handles_unicode_boundaries() {
        let found =
            matches_with_gradient(SearchMode::FuzzyEdit, "café", "xx café yy", Some(0.5)).unwrap();
        assert!(!found.is_empty());
        assert!(found
            .iter()
            .all(|(start, end)| "xx café yy".get(*start..*end).is_some()));
    }
}
