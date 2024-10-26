use super::Opcode;
use std::str::FromStr;

pub fn find_opcode(input: &str) -> Option<Opcode> {
    Some(Opcode::from_u8(find_integer(input, r#""op":"#)?)?)
}

pub fn find_seq(input: &str) -> Option<usize> {
    find_integer(input, r#""s":"#)
}

pub fn find_event_type(input: &str) -> Option<String> {
    let idx = input.find(r#""t":""#)? + r#""t":""#.len();
    let to = input.get(idx..)?.find('"')?;
    Some(input.get(idx..idx + to)?.to_string())
}

fn find_integer<T: FromStr>(input: &str, key: &str) -> Option<T> {
    let idx = input.find(key)? + key.len();
    let to = input.get(idx..)?.find(&[',', '}'] as &[_])?;
    let clean = input.get(idx..idx + to)?.trim();
    T::from_str(clean).ok()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_opcode() {
        assert_eq!(find_opcode(r#"{"op": 3}"#), Some(Opcode::PresenceUpdate));
    }

    #[test]
    fn test_find_event_type() {
        assert_eq!(
            find_event_type(r#"{"s":3,"t":"MESSAGE_CREATE","d": {}q}"#),
            Some("MESSAGE_CREATE".to_string())
        );
    }
}
