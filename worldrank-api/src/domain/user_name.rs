use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct UserName(String);

impl UserName {
    pub fn parse(s: String) -> Result<Self, String> {
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        if s.trim().is_empty() {
            Err(format!("Username {} has no non-whitespace characters.", s))
        } else if s.graphemes(true).count() > 256 {
            Err(format!("Username {} is too long.", s))
        } else if s.chars().any(|g| forbidden_characters.contains(&g)) {
            Err(format!("Username {} contains forbidden characters.", s))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::UserName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a̐".repeat(256);
        assert_ok!(UserName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in vec!['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(UserName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(UserName::parse(name));
    }
}
