//! The closed vocabulary of a request: the position (a FEN), the question's
//! words, the modifiers (the grandmasters' names and a few words that shift
//! the profile), and filler. An unknown word is an error with a hint.

use super::config::Config;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Fen(String),
    /// a modifier by its name in [modifiers]
    Modifier(String),
    Filler,
}

/// Words that ask the question or connect the rest; they carry no tradeoff.
const FILLER: &[&str] = &[
    "what", "is", "the", "best", "move", "in", "for", "find", "solve", "position", "play", "like", "as", "and", "here",
    "mikä", "on", "paras", "siirto", "asemassa", "pelaa", "kuten", "ja",
];

/// Words that mean a modifier.
const WORDS: &[(&str, &str)] = &[
    ("aggressively", "tal"),
    ("attack", "tal"),
    ("hyökkää", "tal"),
    ("safely", "karpov"),
    ("solidly", "karpov"),
    ("varovasti", "karpov"),
    ("simply", "carlsen"),
    ("endgame", "carlsen"),
    ("yksinkertaisesti", "carlsen"),
];

/// Six FEN fields (or four, the counters left out) become one token.
fn take_fen(words: &[&str], i: usize) -> Option<(String, usize)> {
    let w = words.get(i)?;
    if w.matches('/').count() != 7 {
        return None;
    }
    let mut n = 1;
    let mut parts = vec![w.to_string()];
    while n < 6 {
        let Some(x) = words.get(i + n) else { break };
        let field_like = match n {
            1 => matches!(*x, "w" | "b"),
            2 => x.chars().all(|c| "KQkq-".contains(c)),
            3 => *x == "-" || (x.len() == 2 && x.ends_with(|c| c == '3' || c == '6')),
            _ => x.parse::<u32>().is_ok(),
        };
        if !field_like {
            break;
        }
        parts.push(x.to_string());
        n += 1;
    }
    if n < 4 {
        return None;
    }
    Some((parts.join(" "), n))
}

pub fn tokens(sentence: &str, cfg: &Config) -> Result<Vec<Token>, String> {
    let words: Vec<&str> = sentence.split(|c: char| c.is_whitespace() || c == ',' || c == '?').filter(|w| !w.is_empty()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if let Some((fen, n)) = take_fen(&words, i) {
            out.push(Token::Fen(fen));
            i += n;
            continue;
        }
        let lw = words[i].to_lowercase();
        if cfg.modifiers.contains_key(&lw) {
            out.push(Token::Modifier(lw));
        } else if let Some((_, m)) = WORDS.iter().find(|(w, _)| *w == lw) {
            out.push(Token::Modifier(m.to_string()));
        } else if FILLER.contains(&lw.as_str()) {
            out.push(Token::Filler);
        } else {
            return Err(format!("unknown word \"{}\"; the modifiers are {}", words[i], cfg.modifiers.keys().cloned().collect::<Vec<_>>().join(", ")));
        }
        i += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fen_a_name_and_a_word() {
        let cfg = Config::load().unwrap();
        let t = tokens("best move in rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 like Tal, safely", &cfg).unwrap();
        assert!(matches!(&t[3], Token::Fen(f) if f.ends_with("0 1")));
        assert_eq!(t.iter().filter(|x| matches!(x, Token::Modifier(_))).count(), 2);
        assert!(tokens("best move in rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 like stockfish", &cfg).is_err());
    }
}
