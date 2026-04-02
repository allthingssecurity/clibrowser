use std::path::Path;
use std::collections::HashMap;
use anyhow::Result;
use crate::cli::StatsArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::StatsResult;
use crate::output::OutputConfig;

pub fn execute(args: StatsArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file).into()); }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let text = backend.text(path, None)?;

    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len();
    let paragraphs = text.split("\n\n").filter(|p| !p.trim().is_empty()).count().max(1);

    // Sentences: split on .!? followed by space or end
    let sentences: Vec<&str> = text.split(|c: char| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let sentence_count = sentences.len().max(1);

    let avg_sentence_length = word_count as f64 / sentence_count as f64;

    // Unique words and frequency
    let mut freq: HashMap<String, usize> = HashMap::new();
    for w in &words {
        let lower = w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        if !lower.is_empty() {
            *freq.entry(lower).or_insert(0) += 1;
        }
    }
    let unique_words = freq.len();

    let mut top_words: Vec<(String, usize)> = freq.into_iter().collect();
    top_words.sort_by(|a, b| b.1.cmp(&a.1));
    top_words.truncate(20);

    // Flesch-Kincaid Grade Level approximation
    // FK = 0.39 * (words/sentences) + 11.8 * (syllables/words) - 15.59
    let total_syllables: usize = words.iter().map(|w| count_syllables(w)).sum();
    let syllables_per_word = if word_count > 0 { total_syllables as f64 / word_count as f64 } else { 0.0 };
    let reading_level = 0.39 * avg_sentence_length + 11.8 * syllables_per_word - 15.59;
    let reading_level = (reading_level * 10.0).round() / 10.0;

    let result = StatsResult {
        words: word_count,
        sentences: sentence_count,
        paragraphs,
        avg_sentence_length: (avg_sentence_length * 10.0).round() / 10.0,
        reading_level,
        unique_words,
        top_words,
    };

    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("Words:          {}", result.words));
        out.print_human(&format!("Sentences:      {}", result.sentences));
        out.print_human(&format!("Paragraphs:     {}", result.paragraphs));
        out.print_human(&format!("Avg sent. len:  {:.1}", result.avg_sentence_length));
        out.print_human(&format!("Reading level:  {:.1} (Flesch-Kincaid)", result.reading_level));
        out.print_human(&format!("Unique words:   {}", result.unique_words));
        if args.detailed {
            out.print_human("\nTop words:");
            for (w, c) in &result.top_words {
                out.print_human(&format!("  {:20} {}", w, c));
            }
        }
    }
    Ok(0)
}

fn count_syllables(word: &str) -> usize {
    let w = word.to_lowercase();
    let w = w.trim_matches(|c: char| !c.is_alphabetic());
    if w.is_empty() { return 1; }
    let mut count = 0usize;
    let mut prev_vowel = false;
    for ch in w.chars() {
        let is_vowel = "aeiouy".contains(ch);
        if is_vowel && !prev_vowel { count += 1; }
        prev_vowel = is_vowel;
    }
    // Adjust: silent e at end
    if w.ends_with('e') && count > 1 { count -= 1; }
    count.max(1)
}
