#[derive(Debug, Clone, PartialEq)]
pub enum Kv {
    Str(String),
    Block(Vec<(String, Kv)>),
}

#[derive(Debug, thiserror::Error)]
pub enum KvError {
    #[error("unexpected end of input")]
    Eof,
    #[error("unexpected '}}'")]
    StrayClose,
    #[error("a key is missing its value")]
    MissingValue,
}

#[derive(Debug, PartialEq)]
enum Token {
    Open,
    Close,
    Str(String),
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == '/' {
            while i < bytes.len() && bytes[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '{' {
            tokens.push(Token::Open);
            i += 1;
            continue;
        }
        if c == '}' {
            tokens.push(Token::Close);
            i += 1;
            continue;
        }
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < bytes.len() && bytes[i] != '"' {
                if bytes[i] == '\\' && i + 1 < bytes.len() {
                    i += 1;
                    s.push(match bytes[i] {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                } else {
                    s.push(bytes[i]);
                }
                i += 1;
            }
            i += 1;
            tokens.push(Token::Str(s));
            continue;
        }
        let mut s = String::new();
        while i < bytes.len()
            && !bytes[i].is_whitespace()
            && bytes[i] != '{'
            && bytes[i] != '}'
            && bytes[i] != '"'
        {
            s.push(bytes[i]);
            i += 1;
        }
        tokens.push(Token::Str(s));
    }
    tokens
}

pub fn parse(input: &str) -> Result<Vec<(String, Kv)>, KvError> {
    let tokens = tokenize(input);
    let mut pos = 0;
    let pairs = parse_pairs(&tokens, &mut pos, false)?;
    Ok(pairs)
}

fn parse_pairs(
    tokens: &[Token],
    pos: &mut usize,
    in_block: bool,
) -> Result<Vec<(String, Kv)>, KvError> {
    let mut pairs = Vec::new();
    loop {
        match tokens.get(*pos) {
            None => {
                if in_block {
                    return Err(KvError::Eof);
                }
                return Ok(pairs);
            }
            Some(Token::Close) => {
                if !in_block {
                    return Err(KvError::StrayClose);
                }
                *pos += 1;
                return Ok(pairs);
            }
            Some(Token::Open) => return Err(KvError::MissingValue),
            Some(Token::Str(key)) => {
                let key = key.clone();
                *pos += 1;
                match tokens.get(*pos) {
                    Some(Token::Open) => {
                        *pos += 1;
                        let block = parse_pairs(tokens, pos, true)?;
                        pairs.push((key, Kv::Block(block)));
                    }
                    Some(Token::Str(value)) => {
                        let value = value.clone();
                        *pos += 1;
                        pairs.push((key, Kv::Str(value)));
                    }
                    _ => return Err(KvError::MissingValue),
                }
            }
        }
    }
}

impl Kv {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Kv::Str(s) => Some(s.as_str()),
            Kv::Block(_) => None,
        }
    }

    pub fn as_block(&self) -> Option<&[(String, Kv)]> {
        match self {
            Kv::Block(b) => Some(b),
            Kv::Str(_) => None,
        }
    }
}

pub fn find<'a>(pairs: &'a [(String, Kv)], key: &str) -> Option<&'a Kv> {
    pairs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

pub fn get_str<'a>(pairs: &'a [(String, Kv)], key: &str) -> Option<&'a str> {
    find(pairs, key).and_then(|v| v.as_str())
}

pub fn collect<'a>(pairs: &'a [(String, Kv)], key: &str) -> Vec<&'a str> {
    pairs
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case(key))
        .filter_map(|(_, v)| v.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pairs_and_nested_blocks() {
        let text = "\"appbuild\"\n{\n\t\"appid\" \"480\"\n\t\"depots\"\n\t{\n\t\t\"481\" \"depot.vdf\"\n\t}\n}\n";
        let root = parse(text).expect("parse");
        let app = find(&root, "appbuild")
            .and_then(|v| v.as_block())
            .expect("appbuild block");
        assert_eq!(get_str(app, "appid"), Some("480"));
        let depots = find(app, "depots")
            .and_then(|v| v.as_block())
            .expect("depots");
        assert_eq!(get_str(depots, "481"), Some("depot.vdf"));
    }

    #[test]
    fn tolerates_comments_and_odd_whitespace() {
        let text = "// header comment\n\"root\"{   \"a\"\t\t\"1\"  // trailing\n   \"b\"    \"two words\"\n}\n";
        let root = parse(text).expect("parse");
        let b = find(&root, "root").and_then(|v| v.as_block()).unwrap();
        assert_eq!(get_str(b, "a"), Some("1"));
        assert_eq!(get_str(b, "b"), Some("two words"));
    }

    #[test]
    fn accepts_unquoted_tokens() {
        let text = "root { DepotID 481 recursive 1 }";
        let root = parse(text).expect("parse");
        let b = find(&root, "root").and_then(|v| v.as_block()).unwrap();
        assert_eq!(get_str(b, "DepotID"), Some("481"));
        assert_eq!(get_str(b, "recursive"), Some("1"));
    }

    #[test]
    fn collects_repeated_keys() {
        let text = "root { FileExclusion \"*.pdb\" FileExclusion \"*.log\" }";
        let root = parse(text).expect("parse");
        let b = find(&root, "root").and_then(|v| v.as_block()).unwrap();
        assert_eq!(collect(b, "FileExclusion"), vec!["*.pdb", "*.log"]);
    }

    #[test]
    fn reports_unterminated_block() {
        assert!(parse("root { a 1").is_err());
    }
}
