use serde::{Deserialize, Serialize};

/// A syllable with timing and confidence, before it is attached to a note.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Syllable {
    pub text: String,
    /// Start time in seconds.
    pub start_s: f64,
    /// End time in seconds.
    pub end_s: f64,
    pub confidence: f32,
}

impl Syllable {
    pub fn duration_s(&self) -> f64 {
        (self.end_s - self.start_s).max(0.0)
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_defaults_are_empty() {
        let syllable = Syllable::default();
        assert!(syllable.is_empty());
        assert_eq!(syllable.start_s, 0.0);
        assert_eq!(syllable.end_s, 0.0);
        assert_eq!(syllable.confidence, 0.0);
        assert_eq!(syllable.duration_s(), 0.0);
    }

    #[test]
    fn syllable_round_trips_through_json() {
        let syllable = Syllable {
            text: "ción".into(),
            start_s: 1.25,
            end_s: 1.75,
            confidence: 0.82,
        };
        let json = serde_json::to_string(&syllable).expect("serialize");
        let parsed: Syllable = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, syllable);
        assert!((parsed.duration_s() - 0.5).abs() < f64::EPSILON);
    }
}
