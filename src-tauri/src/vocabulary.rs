use std::collections::HashSet;

pub const MAX_TERMS: usize = 50;
pub const MAX_TERM_CHARS: usize = 80;
pub const MAX_TOTAL_CHARS: usize = 2_000;
pub const INVALID_VOCABULARY: &str = "Речникът допуска до 50 термина, по 80 знака, общо 2000 знака. Използвайте по един термин на ред, без <, > или контролни знаци.";

pub fn normalize(terms: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    terms
        .iter()
        .map(|term| term.trim().to_string())
        .filter(|term| !term.is_empty() && seen.insert(term.to_lowercase()))
        .collect()
}

pub fn validate(terms: &[String]) -> Result<(), String> {
    if terms.len() > MAX_TERMS
        || terms.iter().any(|term| {
            term.chars().count() > MAX_TERM_CHARS
                || term
                    .chars()
                    .any(|c| c.is_control() || matches!(c, '<' | '>'))
        })
        || terms.iter().map(|term| term.chars().count()).sum::<usize>() > MAX_TOTAL_CHARS
    {
        return Err(INVALID_VOCABULARY.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_preserve_spelling_and_deduplicate_case_and_empty_lines() {
        assert_eq!(
            normalize(&[
                "  кофердам ".into(),
                "Кофердам".into(),
                " ".into(),
                "специална фраза".into()
            ]),
            vec!["кофердам", "специална фраза"]
        );
    }

    #[test]
    fn limits_count_unicode_characters_and_reject_invalid_keyword_content() {
        assert!(validate(&vec!["я".repeat(40); 50]).is_ok());
        for terms in [
            vec!["я".repeat(81)],
            vec!["дума".into(); 51],
            vec!["я".repeat(41); 50],
            vec!["<tag>".into()],
            vec!["две\nдуми".into()],
            vec!["nul\0".into()],
        ] {
            assert!(validate(&terms).is_err());
        }
    }
}
