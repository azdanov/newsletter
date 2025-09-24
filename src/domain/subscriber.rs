use anyhow::anyhow;
use regex::Regex;
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;
use validator::ValidateEmail;

static RE_VALID_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^[^/()"<>\\{}]+$"#).unwrap());

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, anyhow::Error> {
        if !s.validate_email() {
            return Err(anyhow!("Invalid email format"));
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<Self, anyhow::Error> {
        if s.trim().is_empty() {
            return Err(anyhow!("Name cannot be empty or whitespace"));
        }
        if s.graphemes(true).count() > 256 {
            return Err(anyhow!("Name cannot be longer than 256 characters"));
        }
        if !RE_VALID_NAME.is_match(&s) {
            return Err(anyhow!("Name contains forbidden characters"));
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl NewSubscriber {
    pub fn new(email: String, name: String) -> Result<Self, anyhow::Error> {
        Ok(Self {
            email: SubscriberEmail::parse(email)?,
            name: SubscriberName::parse(name)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_invalid_subscriber() {
        NewSubscriber::new("John".to_string(), "john@example.com".to_string())
            .expect_err("Should fail to create subscriber with invalid email");
    }

    #[test]
    fn create_valid_subscriber() {
        let subscriber = NewSubscriber::new("john@example.com".to_string(), "John".to_string())
            .expect("Failed to create subscriber");
        assert_eq!(subscriber.email.as_ref(), "john@example.com");
        assert_eq!(subscriber.name.as_ref(), "John");
    }

    #[test]
    fn valid_subscriber_passes_validation() {
        let result = NewSubscriber::new("test@example.com".to_string(), "John Doe".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_email_fails_validation() {
        let result = NewSubscriber::new("invalid-email".to_string(), "John Doe".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn empty_email_fails_validation() {
        let result = NewSubscriber::new("".to_string(), "John Doe".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn empty_name_fails_validation() {
        let result = NewSubscriber::new("test@example.com".to_string(), "".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn name_too_long_fails_validation() {
        let result = NewSubscriber::new("test@example.com".to_string(), "a".repeat(257));
        assert!(result.is_err());
    }

    #[test]
    fn name_with_forbidden_characters_fails_validation() {
        let forbidden_chars = vec!['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        for char in forbidden_chars {
            let result =
                NewSubscriber::new("test@example.com".to_string(), format!("John{}Doe", char));
            assert!(
                result.is_err(),
                "Name with '{}' should fail validation",
                char
            );
        }
    }

    #[test]
    fn name_with_allowed_characters_passes_validation() {
        let allowed_names = vec![
            "John Doe",
            "Mary-Jane",
            "José María",
            "O'Connor",
            "李小明",
            "123",
            "user@domain",
            "Name with spaces and numbers 123",
        ];

        for name in allowed_names {
            let result = NewSubscriber::new("test@example.com".to_string(), name.to_string());
            assert!(result.is_ok(), "Name '{}' should pass validation", name);
        }
    }

    #[test]
    fn name_at_max_length_passes_validation() {
        let result = NewSubscriber::new("test@example.com".to_string(), "a".repeat(256));
        assert!(result.is_ok());
    }
}
