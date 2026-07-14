//! Development-only characterization of local phonetic representations.
//!
//! This example is intentionally disconnected from the VoxProof library,
//! CLI, candidate pipeline, and review semantics. It records dependency
//! behavior for design review; it is not a detector or a quality benchmark.

use std::collections::BTreeSet;

use pinyin::{Pinyin, ToPinyin, ToPinyinMulti};
use rphonetic::DoubleMetaphone;

const ROMAN_EXPANSION_LIMIT: usize = 16_384;

const INPUTS: &[&str] = &[
    "PostgreSQL",
    "postgresql",
    "POSTGRESQL",
    "Postgre SQL",
    "Postgre sequel",
    "Postgres",
    "SQL",
    "S Q L",
    "sequel",
    "Kafka",
    "卡夫卡",
    "ka fu ka",
    "Kubernetes",
    "cuber netties",
    "K8s",
    "SHIBUYA SKY",
    "澀谷 Sky",
    "涩谷 Sky",
    "重慶",
    "重庆",
    "銀行",
    "银行",
    "長安",
    "长安",
    "音樂",
    "音乐",
    "快樂",
    "快乐",
    "C#",
    "C++",
    "C sharp",
    "C plus plus",
    "GPU",
    "G P U",
    "API",
    "A P I",
    "卡夫卡 Kafka",
    "Postgre SQL 資料庫",
    "K8s 叢集",
    "camera",
    "postal",
    "copper",
    "咖啡",
    "scale",
    "happy",
    "🙂",
];

const PAIRS: &[PairSpec] = &[
    PairSpec::positive(
        "PostgreSQL",
        "Postgre sequel",
        "useful raw/DM signal; DM identity is not discriminative by itself",
    ),
    PairSpec::positive(
        "PostgreSQL",
        "Postgre SQL",
        "useful after separator removal; token boundaries differ",
    ),
    PairSpec::positive(
        "SQL",
        "sequel",
        "DM collision is useful here but also collides with negative scale",
    ),
    PairSpec::positive(
        "SQL",
        "S Q L",
        "useful separator variant; does not model spoken letter names",
    ),
    PairSpec::positive(
        "Kafka",
        "卡夫卡",
        "useful only after Han-to-pinyin romanization",
    ),
    PairSpec::positive(
        "Kafka",
        "ka fu ka",
        "useful flattened romanized and DM signal",
    ),
    PairSpec::positive(
        "Kubernetes",
        "cuber netties",
        "useful signal; DM truncation also overmatches negative copper",
    ),
    PairSpec::positive(
        "Kubernetes",
        "K8s",
        "weak: phonetic representations do not bridge the abbreviation",
    ),
    PairSpec::positive(
        "SHIBUYA SKY",
        "澀谷 Sky",
        "weak: Mandarin pinyin does not represent Japanese Shibuya",
    ),
    PairSpec::positive(
        "SHIBUYA SKY",
        "涩谷 Sky",
        "weak: Mandarin pinyin does not represent Japanese Shibuya",
    ),
    PairSpec::positive(
        "重慶",
        "重庆",
        "useful script-variant identity, but both primary readings are contextually wrong",
    ),
    PairSpec::positive(
        "銀行",
        "银行",
        "useful script-variant identity, but primary xing is wrong for the phrase",
    ),
    PairSpec::positive(
        "音樂",
        "音乐",
        "useful script-variant identity, but primary le is wrong for the phrase",
    ),
    PairSpec::positive(
        "快樂",
        "快乐",
        "useful script-variant identity and expected primary reading",
    ),
    PairSpec::positive(
        "GPU",
        "G P U",
        "useful separator variant; not a spoken-letter representation",
    ),
    PairSpec::positive(
        "API",
        "A P I",
        "useful separator variant; not a spoken-letter representation",
    ),
    PairSpec::positive(
        "C#",
        "C sharp",
        "positive terminology/pronunciation relation, but generic distance loses # identity; explicit domain knowledge may be required",
    ),
    PairSpec::positive(
        "C++",
        "C plus plus",
        "positive terminology/pronunciation relation, but generic distance loses + identity; explicit domain knowledge may be required",
    ),
    PairSpec::negative(
        "C++",
        "C sharp",
        "negative control: generic normalization collapses C++ and C# to c and cannot distinguish the symbolic names",
    ),
    PairSpec::negative(
        "Kafka",
        "camera",
        "useful separation in tested representations",
    ),
    PairSpec::negative(
        "PostgreSQL",
        "postal",
        "DM is misleadingly close after four-key truncation",
    ),
    PairSpec::negative(
        "Kubernetes",
        "copper",
        "DM is misleadingly close after four-key truncation",
    ),
    PairSpec::negative(
        "卡夫卡",
        "咖啡",
        "all-reading expansion increases a false romanized resemblance",
    ),
    PairSpec::negative(
        "SQL",
        "scale",
        "misleading: exact Double Metaphone collision",
    ),
    PairSpec::negative(
        "API",
        "happy",
        "weak DM resemblance but no exact key collision",
    ),
];

#[derive(Clone, Copy)]
struct PairSpec {
    left: &'static str,
    right: &'static str,
    expected_relation: &'static str,
    interpretation: &'static str,
}

impl PairSpec {
    const fn positive(
        left: &'static str,
        right: &'static str,
        interpretation: &'static str,
    ) -> Self {
        Self {
            left,
            right,
            expected_relation: "positive",
            interpretation,
        }
    }

    const fn negative(
        left: &'static str,
        right: &'static str,
        interpretation: &'static str,
    ) -> Self {
        Self {
            left,
            right,
            expected_relation: "negative",
            interpretation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Atom {
    kind: &'static str,
    surface: String,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HanReading {
    surface: char,
    start_byte: usize,
    end_byte: usize,
    primary_plain: String,
    primary_tone: String,
    all_plain: Vec<String>,
    all_tone: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InputRecord {
    original: String,
    atoms: Vec<Atom>,
    raw_normalized: String,
    han_readings: Vec<HanReading>,
    primary_roman_tokens: Vec<String>,
    primary_roman_flat: String,
    primary_roman_tone_flat: String,
    roman_expansion: ExpansionResult,
    han_tone_reading_combinations: Option<usize>,
    dm_primary: String,
    dm_alternate: String,
    dm_full_primary: String,
    dm_full_alternate: String,
    dm_status: &'static str,
    non_pinyin_atoms: Vec<String>,
    unsupported_atoms: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExpansionResult {
    generated: Vec<String>,
    theoretical_count: usize,
    limit: usize,
    truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Score {
    distance: usize,
    numerator: usize,
    denominator: usize,
    permille: usize,
}

fn main() {
    let records = INPUTS
        .iter()
        .map(|input| analyze(input))
        .collect::<Vec<_>>();

    println!("# Deterministic phonetic representation characterization");
    println!();
    println!("Dependencies: pinyin 0.11.0; rphonetic 3.0.6.");
    println!(
        "Atomization classifies pinyin-supported characters as Han, then ASCII Latin, digits, whitespace, punctuation, and unsupported input; it is not a complete Unicode Script implementation."
    );
    println!(
        "Original surfaces and byte ranges remain recorded. Lowercasing and separator removal occur only in derived experiment representations and are not product case or presentation policy."
    );
    println!(
        "Double Metaphone is evaluated only for ASCII inputs in the normal report. The reproducible rphonetic 3.0.6 mixed-UTF-8 panic is isolated in one dedicated test."
    );
    println!(
        "Symbolic-name relations such as C#/C sharp and C++/C plus plus are terminology cases that may require explicit domain knowledge; generic distance is characterized but not treated as their solution."
    );
    println!(
        "Normalized score formula: (max(unit_len_left, unit_len_right) - edit_distance) / max(unit_len_left, unit_len_right)."
    );
    println!(
        "The report shows the exact rational and a floor-rounded permille value; these are similarities, not confidence."
    );
    println!();
    print_input_table(&records);
    println!();
    print_pair_table();
    println!();
    print_polyphonic_table(&records);
}

fn analyze(input: &str) -> InputRecord {
    let atoms = atomize(input);
    let han_readings = han_readings(input);
    let choices = roman_choices(&atoms);
    let roman_expansion = bounded_cartesian_flatten(&choices, ROMAN_EXPANSION_LIMIT);
    let primary_roman_tokens = choices
        .iter()
        .filter_map(|choice| choice.first().cloned())
        .collect::<Vec<_>>();
    let primary_roman_flat = primary_roman_tokens.concat();
    let primary_roman_tone_flat = atoms
        .iter()
        .filter_map(|atom| match atom.kind {
            "han" => atom
                .surface
                .chars()
                .next()
                .and_then(|ch| ch.to_pinyin())
                .map(|pinyin| pinyin.with_tone_num_end().to_string()),
            "latin" => Some(atom.surface.to_ascii_lowercase()),
            "digit" => Some(atom.surface.clone()),
            "space" | "punct" | "unsupported" => None,
            _ => unreachable!("known atom kind"),
        })
        .collect::<String>();
    let han_tone_reading_combinations = (!han_readings.is_empty()).then(|| {
        han_readings
            .iter()
            .map(|reading| reading.all_tone.len())
            .fold(1usize, |product, count| product.saturating_mul(count))
    });
    let non_pinyin_atoms = atoms
        .iter()
        .filter(|atom| matches!(atom.kind, "latin" | "digit" | "punct"))
        .map(|atom| format!("{}:{}", atom.kind, display_surface(&atom.surface)))
        .collect();
    let unsupported_atoms = atoms
        .iter()
        .filter(|atom| atom.kind == "unsupported")
        .map(|atom| format!("{}:{}", atom.kind, display_surface(&atom.surface)))
        .collect();

    let (dm_primary, dm_alternate, dm_full_primary, dm_full_alternate, dm_status) =
        if input.is_ascii() {
            let (dm_primary, dm_alternate) =
                double_metaphone_outputs(DoubleMetaphone::default(), input);
            let (dm_full_primary, dm_full_alternate) =
                double_metaphone_outputs(DoubleMetaphone::new(None), input);
            (
                dm_primary,
                dm_alternate,
                dm_full_primary,
                dm_full_alternate,
                "ok",
            )
        } else {
            (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "skipped-non-ascii",
            )
        };

    InputRecord {
        original: input.to_string(),
        atoms,
        raw_normalized: normalize_raw(input),
        han_readings,
        primary_roman_tokens,
        primary_roman_flat,
        primary_roman_tone_flat,
        roman_expansion,
        han_tone_reading_combinations,
        dm_primary,
        dm_alternate,
        dm_full_primary,
        dm_full_alternate,
        dm_status,
        non_pinyin_atoms,
        unsupported_atoms,
    }
}

fn double_metaphone_outputs(encoder: DoubleMetaphone, input: &str) -> (String, String) {
    let result = encoder.double_metaphone(input);
    (result.primary(), result.alternate())
}

fn atomize(input: &str) -> Vec<Atom> {
    let mut atoms: Vec<Atom> = Vec::new();

    for (start_byte, ch) in input.char_indices() {
        let end_byte = start_byte + ch.len_utf8();
        let kind = char_kind(ch);

        if matches!(kind, "latin" | "digit" | "space")
            && atoms.last().is_some_and(|last| last.kind == kind)
        {
            let last = atoms.last_mut().expect("checked above");
            last.surface.push(ch);
            last.end_byte = end_byte;
        } else {
            atoms.push(Atom {
                kind,
                surface: ch.to_string(),
                start_byte,
                end_byte,
            });
        }
    }

    atoms
}

fn char_kind(ch: char) -> &'static str {
    if ch.to_pinyin().is_some() {
        "han"
    } else if ch.is_ascii_alphabetic() {
        "latin"
    } else if ch.is_ascii_digit() {
        "digit"
    } else if ch.is_whitespace() {
        "space"
    } else if ch.is_ascii_punctuation() {
        "punct"
    } else {
        "unsupported"
    }
}

fn han_readings(input: &str) -> Vec<HanReading> {
    input
        .char_indices()
        .filter_map(|(start_byte, ch)| {
            let primary = ch.to_pinyin()?;
            let all = ch.to_pinyin_multi()?;
            Some(HanReading {
                surface: ch,
                start_byte,
                end_byte: start_byte + ch.len_utf8(),
                primary_plain: primary.plain().to_string(),
                primary_tone: primary.with_tone_num_end().to_string(),
                all_plain: unique(all.into_iter().map(Pinyin::plain)),
                all_tone: unique(all.into_iter().map(Pinyin::with_tone_num_end)),
            })
        })
        .collect()
}

fn roman_choices(atoms: &[Atom]) -> Vec<Vec<String>> {
    atoms
        .iter()
        .filter_map(|atom| match atom.kind {
            "han" => {
                let ch = atom.surface.chars().next().expect("one Han character");
                let all = ch.to_pinyin_multi().expect("Han atom has pinyin");
                Some(unique(all.into_iter().map(Pinyin::plain)))
            }
            "latin" => Some(vec![atom.surface.to_ascii_lowercase()]),
            "digit" => Some(vec![atom.surface.clone()]),
            "space" | "punct" | "unsupported" => None,
            _ => unreachable!("known atom kind"),
        })
        .collect()
}

fn unique<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .filter_map(|value| {
            if seen.insert(value) {
                Some(value.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn bounded_cartesian_flatten(choices: &[Vec<String>], limit: usize) -> ExpansionResult {
    if choices.is_empty() {
        return ExpansionResult {
            generated: Vec::new(),
            theoretical_count: 0,
            limit,
            truncated: false,
        };
    }

    let theoretical_count = choices.iter().fold(1usize, |product, choice| {
        product.saturating_mul(choice.len())
    });
    let mut generated = Vec::with_capacity(limit.min(theoretical_count));
    generate_combinations(choices, 0, String::new(), limit, &mut generated);

    ExpansionResult {
        truncated: generated.len() < theoretical_count,
        generated,
        theoretical_count,
        limit,
    }
}

fn generate_combinations(
    choices: &[Vec<String>],
    choice_index: usize,
    prefix: String,
    limit: usize,
    generated: &mut Vec<String>,
) {
    if generated.len() == limit {
        return;
    }
    if choice_index == choices.len() {
        generated.push(prefix);
        return;
    }

    for value in &choices[choice_index] {
        if generated.len() == limit {
            break;
        }
        let mut combined = prefix.clone();
        combined.push_str(value);
        generate_combinations(choices, choice_index + 1, combined, limit, generated);
    }
}

fn normalize_raw(input: &str) -> String {
    input
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn levenshtein_chars(left: &str, right: &str) -> usize {
    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    levenshtein(&left, &right)
}

fn levenshtein<T: Eq>(left: &[T], right: &[T]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];

    for (left_index, left_value) in left.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_value) in right.iter().enumerate() {
            let substitution = previous[right_index] + usize::from(left_value != right_value);
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            current[right_index + 1] = substitution.min(insertion).min(deletion);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

fn score_chars(left: &str, right: &str) -> Option<Score> {
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let denominator = left.chars().count().max(right.chars().count());
    let distance = levenshtein_chars(left, right);
    Some(score(distance, denominator))
}

fn score_tokens(left: &[String], right: &[String]) -> Option<Score> {
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let denominator = left.len().max(right.len());
    let distance = levenshtein(left, right);
    Some(score(distance, denominator))
}

fn score(distance: usize, denominator: usize) -> Score {
    debug_assert!(denominator > 0);
    let numerator = denominator.saturating_sub(distance);
    Score {
        distance,
        numerator,
        denominator,
        permille: numerator * 1000 / denominator,
    }
}

fn best_score(left: &[String], right: &[String]) -> Option<Score> {
    left.iter()
        .flat_map(|left_value| {
            right
                .iter()
                .filter_map(move |right_value| score_chars(left_value, right_value))
        })
        .max_by_key(|score| (score.permille, usize::MAX - score.distance))
}

fn best_dm_score(left: &InputRecord, right: &InputRecord) -> Option<Score> {
    best_key_score(
        [&left.dm_primary, &left.dm_alternate],
        [&right.dm_primary, &right.dm_alternate],
    )
}

fn best_full_dm_score(left: &InputRecord, right: &InputRecord) -> Option<Score> {
    best_key_score(
        [&left.dm_full_primary, &left.dm_full_alternate],
        [&right.dm_full_primary, &right.dm_full_alternate],
    )
}

fn best_key_score(left_keys: [&String; 2], right_keys: [&String; 2]) -> Option<Score> {
    left_keys
        .into_iter()
        .flat_map(|left_key| {
            right_keys
                .into_iter()
                .filter(|right_key| !left_key.is_empty() && !right_key.is_empty())
                .filter_map(move |right_key| score_chars(left_key, right_key))
        })
        .max_by_key(|score| (score.permille, usize::MAX - score.distance))
}

fn any_dm_key_equal(left: &InputRecord, right: &InputRecord) -> bool {
    any_key_equal(
        [&left.dm_primary, &left.dm_alternate],
        [&right.dm_primary, &right.dm_alternate],
    )
}

fn any_full_dm_key_equal(left: &InputRecord, right: &InputRecord) -> bool {
    any_key_equal(
        [&left.dm_full_primary, &left.dm_full_alternate],
        [&right.dm_full_primary, &right.dm_full_alternate],
    )
}

fn any_key_equal(left_keys: [&String; 2], right_keys: [&String; 2]) -> bool {
    left_keys.into_iter().any(|left_key| {
        right_keys
            .into_iter()
            .any(|right_key| !left_key.is_empty() && left_key == right_key)
    })
}

fn print_input_table(records: &[InputRecord]) {
    println!("## Per-input outputs");
    println!(
        "| original | atoms (`kind:surface@bytes`) | raw normalized | primary pinyin (tone) | all pinyin readings | primary roman tokens | normalized romanized (toneless / tone-aware) | DM4 primary / alternate | DM-unbounded primary / alternate | DM status | non-pinyin atoms | genuinely unsupported atoms | Han tone-reading combinations | toneless expansion | deterministic repeat |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---:|---|---|");

    for record in records {
        let repeated = analyze(&record.original) == *record;
        println!(
            "| {} | {} | {} | {} | {} | {} | {} / {} | {} / {} | {} / {} | {} | {} | {} | {} | {} | {} |",
            escape(&record.original),
            escape(&format_atoms(&record.atoms)),
            escape(&record.raw_normalized),
            escape(&format_primary_pinyin(&record.han_readings)),
            escape(&format_all_pinyin(&record.han_readings)),
            escape(&record.primary_roman_tokens.join(" · ")),
            escape(&record.primary_roman_flat),
            escape(&record.primary_roman_tone_flat),
            escape(&record.dm_primary),
            escape(&record.dm_alternate),
            escape(&record.dm_full_primary),
            escape(&record.dm_full_alternate),
            record.dm_status,
            escape(&record.non_pinyin_atoms.join(" · ")),
            escape(&record.unsupported_atoms.join(" · ")),
            format_optional_count(record.han_tone_reading_combinations),
            format_expansion(&record.roman_expansion),
            repeated,
        );
    }
}

fn print_pair_table() {
    println!("## Pairwise outputs");
    println!(
        "| pair | expected | raw representations | raw d; score | primary roman representations | char d; score | token d; score | best all-reading score | DM4 primary keys | primary d; score | best primary/alternate score | DM4 exact primary / any key | DM-unbounded primary keys | best primary/alternate score | DM-unbounded any exact key | interpretation |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");

    for pair in PAIRS {
        let left = analyze(pair.left);
        let right = analyze(pair.right);
        let raw = score_chars(&left.raw_normalized, &right.raw_normalized);
        let roman = score_chars(&left.primary_roman_flat, &right.primary_roman_flat);
        let token = score_tokens(&left.primary_roman_tokens, &right.primary_roman_tokens);
        let best_roman = best_score(
            &left.roman_expansion.generated,
            &right.roman_expansion.generated,
        );
        let dm_primary = score_chars(&left.dm_primary, &right.dm_primary);
        let dm_best = best_dm_score(&left, &right);
        let dm_full_best = best_full_dm_score(&left, &right);
        let exact_primary = !left.dm_primary.is_empty() && left.dm_primary == right.dm_primary;

        println!(
            "| {} ↔ {} | {} | {} ↔ {} | {} | {} ↔ {} | {} | {} | {} | {} ↔ {} | {} | {} | {} / {} | {} ↔ {} | {} | {} | {} |",
            escape(pair.left),
            escape(pair.right),
            pair.expected_relation,
            escape(&left.raw_normalized),
            escape(&right.raw_normalized),
            format_optional_score(raw),
            escape(&left.primary_roman_flat),
            escape(&right.primary_roman_flat),
            format_optional_score(roman),
            format_optional_score(token),
            format_optional_score(best_roman),
            escape(&left.dm_primary),
            escape(&right.dm_primary),
            format_optional_score(dm_primary),
            format_optional_score(dm_best),
            exact_primary,
            any_dm_key_equal(&left, &right),
            escape(&left.dm_full_primary),
            escape(&right.dm_full_primary),
            format_optional_score(dm_full_best),
            any_full_dm_key_equal(&left, &right),
            pair.interpretation,
        );
    }
}

fn print_polyphonic_table(records: &[InputRecord]) {
    println!("## Polyphonic-character outputs");
    println!(
        "| phrase | character reading lists (`primary; all`) | Han tone-reading combinations | toneless expansion | primary phrase | expected phrase reading (interpretive) | primary matches expected |"
    );
    println!("|---|---|---:|---|---|---|---|");

    for phrase in ["卡夫卡", "重慶", "銀行", "長安", "音樂", "快樂"] {
        let record = records
            .iter()
            .find(|record| record.original == phrase)
            .expect("polyphonic phrase is in input list");
        println!(
            "| {} | {} | {} | {} | {} | {} | {} |",
            phrase,
            escape(&format_all_pinyin(&record.han_readings)),
            format_optional_count(record.han_tone_reading_combinations),
            format_expansion(&record.roman_expansion),
            escape(&record.primary_roman_tokens.join(" ")),
            expected_phrase_reading(phrase),
            record.primary_roman_tokens.join(" ") == expected_phrase_reading(phrase),
        );
    }
}

fn expected_phrase_reading(phrase: &str) -> &'static str {
    match phrase {
        "卡夫卡" => "ka fu ka",
        "重慶" => "chong qing",
        "銀行" => "yin hang",
        "長安" => "chang an",
        "音樂" => "yin yue",
        "快樂" => "kuai le",
        _ => unreachable!("known polyphonic phrase"),
    }
}

fn format_score(score: Score) -> String {
    format!(
        "d={}; {}/{}; {}/1000",
        score.distance, score.numerator, score.denominator, score.permille
    )
}

fn format_optional_score(score: Option<Score>) -> String {
    score.map_or_else(|| "n/a".to_string(), format_score)
}

fn format_optional_count(count: Option<usize>) -> String {
    count.map_or_else(|| "n/a".to_string(), |count| count.to_string())
}

fn format_expansion(expansion: &ExpansionResult) -> String {
    format!(
        "generated={}; upper_bound={}; limit={}; truncated={}",
        expansion.generated.len(),
        expansion.theoretical_count,
        expansion.limit,
        expansion.truncated
    )
}

fn format_atoms(atoms: &[Atom]) -> String {
    atoms
        .iter()
        .map(|atom| {
            format!(
                "{}:{}@{}..{}",
                atom.kind,
                display_surface(&atom.surface),
                atom.start_byte,
                atom.end_byte
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn format_primary_pinyin(readings: &[HanReading]) -> String {
    readings
        .iter()
        .map(|reading| {
            format!(
                "{}@{}..{}={};{}",
                reading.surface,
                reading.start_byte,
                reading.end_byte,
                reading.primary_plain,
                reading.primary_tone
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn format_all_pinyin(readings: &[HanReading]) -> String {
    readings
        .iter()
        .map(|reading| {
            format!(
                "{}={} [plain:{}; tone:{}]",
                reading.surface,
                reading.primary_plain,
                reading.all_plain.join(","),
                reading.all_tone.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn display_surface(surface: &str) -> String {
    surface.replace(' ', "␠")
}

fn escape(value: &str) -> String {
    value.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_characterization_input_is_deterministic() {
        for input in INPUTS {
            assert_eq!(analyze(input), analyze(input), "{input}");
        }
    }

    #[test]
    fn required_categories_and_pairs_are_present() {
        for input in [
            "重慶",
            "重庆",
            "銀行",
            "银行",
            "長安",
            "长安",
            "音樂",
            "音乐",
            "快樂",
            "快乐",
            "SQL",
            "S Q L",
            "C#",
            "C++",
            "C sharp",
            "C plus plus",
            "澀谷 Sky",
            "Postgre SQL 資料庫",
        ] {
            assert!(INPUTS.contains(&input), "missing input: {input}");
        }

        for (left, right) in [
            ("PostgreSQL", "Postgre sequel"),
            ("SQL", "S Q L"),
            ("Kafka", "卡夫卡"),
            ("Kubernetes", "cuber netties"),
            ("SHIBUYA SKY", "涩谷 Sky"),
            ("卡夫卡", "咖啡"),
            ("C#", "C sharp"),
            ("C++", "C plus plus"),
            ("C++", "C sharp"),
        ] {
            assert!(
                PAIRS
                    .iter()
                    .any(|pair| pair.left == left && pair.right == right),
                "missing pair: {left} / {right}"
            );
        }

        for (left, right, expected_relation) in [
            ("C#", "C sharp", "positive"),
            ("C++", "C plus plus", "positive"),
            ("C++", "C sharp", "negative"),
        ] {
            let pair = PAIRS
                .iter()
                .find(|pair| pair.left == left && pair.right == right)
                .expect("C-family pair should exist");
            assert_eq!(pair.expected_relation, expected_relation);
        }
    }

    #[test]
    fn score_formula_is_exact_and_not_floating_point() {
        assert_eq!(
            score_chars("abc", "adc"),
            Some(Score {
                distance: 1,
                numerator: 2,
                denominator: 3,
                permille: 666,
            })
        );
    }

    #[test]
    fn empty_or_unsupported_comparisons_are_unavailable() {
        assert_eq!(score_chars("", ""), None);
        assert_eq!(score_chars("", "abc"), None);
        assert_eq!(score_tokens(&[], &[]), None);
        assert_eq!(best_score(&["".to_string()], &["".to_string()]), None);

        let unsupported = analyze("🙂");
        assert!(unsupported.raw_normalized.is_empty());
        assert!(unsupported.primary_roman_flat.is_empty());
        assert_eq!(unsupported.unsupported_atoms, ["unsupported:🙂"]);
        assert!(unsupported.roman_expansion.generated.is_empty());
        assert_eq!(unsupported.roman_expansion.theoretical_count, 0);
        assert_eq!(format_optional_score(score_chars("", "")), "n/a");
    }

    #[test]
    fn bounded_expansion_reports_truncation_without_partial_combinations() {
        let choices = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["1".to_string(), "2".to_string()],
        ];
        let expansion = bounded_cartesian_flatten(&choices, 2);

        assert_eq!(expansion.generated, ["a1", "a2"]);
        assert_eq!(expansion.theoretical_count, 4);
        assert_eq!(expansion.limit, 2);
        assert!(expansion.truncated);
    }

    #[test]
    fn punctuation_and_byte_offsets_remain_inspectable() {
        let c_plus_plus = analyze("C++");
        assert_eq!(
            c_plus_plus.atoms,
            vec![
                Atom {
                    kind: "latin",
                    surface: "C".to_string(),
                    start_byte: 0,
                    end_byte: 1,
                },
                Atom {
                    kind: "punct",
                    surface: "+".to_string(),
                    start_byte: 1,
                    end_byte: 2,
                },
                Atom {
                    kind: "punct",
                    surface: "+".to_string(),
                    start_byte: 2,
                    end_byte: 3,
                },
            ]
        );

        let mixed = analyze("澀谷 Sky");
        assert_eq!(mixed.atoms[0].start_byte, 0);
        assert_eq!(mixed.atoms[0].end_byte, 3);
        assert_eq!(mixed.atoms[1].start_byte, 3);
        assert_eq!(mixed.atoms[1].end_byte, 6);
        assert_eq!(mixed.atoms[3].surface, "Sky");
    }

    #[test]
    fn pinyin_0_11_outputs_are_locked_for_selected_polyphonic_cases() {
        let kafka = analyze("卡夫卡");
        assert_eq!(kafka.primary_roman_tokens, ["ka", "fu", "ka"]);
        assert_eq!(kafka.primary_roman_tone_flat, "ka3fu1ka3");
        assert_eq!(kafka.han_readings[0].all_tone, ["ka3", "qia3"]);
        assert_eq!(kafka.han_readings[1].all_tone, ["fu1", "fu2"]);
        assert_eq!(kafka.han_tone_reading_combinations, Some(8));
        assert_eq!(kafka.roman_expansion.generated.len(), 4);
        assert!(!kafka.roman_expansion.truncated);

        let chongqing = analyze("重慶");
        assert_eq!(chongqing.primary_roman_tokens, ["zhong", "qing"]);
        assert_eq!(
            chongqing.han_readings[0].all_tone,
            ["zhong4", "chong2", "tong2"]
        );
        assert_eq!(
            chongqing.han_readings[1].all_tone,
            ["qing4", "qing1", "qiang1"]
        );
        assert_eq!(chongqing.han_tone_reading_combinations, Some(9));
        assert_eq!(chongqing.roman_expansion.generated.len(), 6);

        let bank = analyze("銀行");
        assert_eq!(bank.primary_roman_tokens, ["yin", "xing"]);
        assert_eq!(
            bank.han_readings[1].all_tone,
            ["xing2", "hang2", "heng2", "xing4", "hang4"]
        );
        assert_eq!(bank.han_tone_reading_combinations, Some(5));
        assert_eq!(bank.roman_expansion.generated.len(), 3);

        let music = analyze("音樂");
        assert_eq!(music.primary_roman_tokens, ["yin", "le"]);
        assert_eq!(
            music.han_readings[1].all_tone,
            ["le4", "yue4", "yao4", "luo4", "liao2"]
        );
    }

    #[test]
    fn traditional_and_simplified_outputs_are_character_level_not_phrase_aware() {
        for (traditional, simplified) in [
            ("重慶", "重庆"),
            ("銀行", "银行"),
            ("音樂", "音乐"),
            ("快樂", "快乐"),
        ] {
            let traditional = analyze(traditional);
            let simplified = analyze(simplified);
            assert_ne!(traditional.raw_normalized, simplified.raw_normalized);
            assert_eq!(
                traditional.primary_roman_flat,
                simplified.primary_roman_flat
            );
        }

        assert_eq!(analyze("重慶").primary_roman_tokens, ["zhong", "qing"]);
        assert_eq!(expected_phrase_reading("重慶"), "chong qing");
        assert_eq!(analyze("音樂").primary_roman_tokens, ["yin", "le"]);
        assert_eq!(expected_phrase_reading("音樂"), "yin yue");
        assert_eq!(analyze("快樂").primary_roman_tokens, ["kuai", "le"]);
        assert_eq!(expected_phrase_reading("快樂"), "kuai le");
    }

    #[test]
    fn rphonetic_3_0_6_ascii_outputs_and_non_ascii_skip_are_locked() {
        let postgres = analyze("PostgreSQL");
        assert_eq!(
            (
                postgres.dm_primary.as_str(),
                postgres.dm_alternate.as_str(),
                postgres.dm_full_primary.as_str(),
                postgres.dm_full_alternate.as_str(),
            ),
            ("PSTK", "PSTK", "PSTKRSKL", "PSTKRSKL")
        );

        for input in ["SQL", "sequel", "scale"] {
            let record = analyze(input);
            assert_eq!(record.dm_primary, "SKL", "{input}");
            assert_eq!(record.dm_alternate, "SKL", "{input}");
        }

        assert_eq!(analyze("Kubernetes").dm_primary, "KPRN");
        assert_eq!(analyze("cuber netties").dm_primary, "KPRN");
        assert_eq!(analyze("copper").dm_primary, "KPR");

        let han = analyze("卡夫卡");
        assert_eq!(han.dm_status, "skipped-non-ascii");
        assert!(han.dm_primary.is_empty());

        let mixed = analyze("澀谷 Sky");
        assert_eq!(mixed.dm_status, "skipped-non-ascii");
        assert!(mixed.dm_primary.is_empty());

        let formerly_panicking = analyze("K8s 叢集");
        assert_eq!(formerly_panicking.dm_status, "skipped-non-ascii");
        assert!(formerly_panicking.dm_primary.is_empty());
    }

    #[test]
    fn rphonetic_3_0_6_known_mixed_utf8_panic_is_isolated() {
        let result =
            std::panic::catch_unwind(|| DoubleMetaphone::default().double_metaphone("K8s 叢集"));
        assert!(
            result.is_err(),
            "the locked characterization expects the known rphonetic 3.0.6 panic"
        );
    }

    #[test]
    fn all_pair_scores_are_exercised_with_negative_controls() {
        for pair in PAIRS {
            let left = analyze(pair.left);
            let right = analyze(pair.right);
            let first = (
                score_chars(&left.raw_normalized, &right.raw_normalized),
                score_chars(&left.primary_roman_flat, &right.primary_roman_flat),
                score_tokens(&left.primary_roman_tokens, &right.primary_roman_tokens),
                best_score(
                    &left.roman_expansion.generated,
                    &right.roman_expansion.generated,
                ),
                best_dm_score(&left, &right),
            );
            let second = (
                score_chars(&left.raw_normalized, &right.raw_normalized),
                score_chars(&left.primary_roman_flat, &right.primary_roman_flat),
                score_tokens(&left.primary_roman_tokens, &right.primary_roman_tokens),
                best_score(
                    &left.roman_expansion.generated,
                    &right.roman_expansion.generated,
                ),
                best_dm_score(&left, &right),
            );
            assert_eq!(first, second, "{} / {}", pair.left, pair.right);
        }

        assert_eq!(analyze("C++").raw_normalized, "c");
        assert_eq!(analyze("C#").raw_normalized, "c");
        assert_eq!(
            analyze("C++").non_pinyin_atoms,
            ["latin:C", "punct:+", "punct:+"]
        );
        assert_eq!(analyze("🙂").unsupported_atoms, ["unsupported:🙂"]);
    }
}
