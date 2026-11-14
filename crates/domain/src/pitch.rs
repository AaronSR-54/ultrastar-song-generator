use crate::song::Pitch;

/// Lowest representable MIDI note (0) shifted to UltraStar pitch.
pub const PITCH_MIN: Pitch = -60;
/// Highest representable MIDI note (127) shifted to UltraStar pitch.
pub const PITCH_MAX: Pitch = 67;

/// Converts a frequency in hertz to a fractional MIDI note.
pub fn hz_to_midi(hz: f64) -> f64 {
    69.0 + 12.0 * (hz / 440.0).log2()
}

/// Converts a frequency in hertz to the integer UltraStar pitch.
///
/// UltraStar pitch 0 is MIDI 60 (C4).
pub fn hz_to_pitch(hz: f64) -> Pitch {
    hz_to_midi(hz).round() as Pitch - 60
}

/// Converts an UltraStar pitch to its MIDI note number.
pub fn pitch_to_midi(pitch: Pitch) -> Pitch {
    pitch + 60
}

/// Converts a MIDI note number to its frequency in hertz.
pub fn midi_to_hz(midi: f64) -> f64 {
    440.0 * 2.0_f64.powf((midi - 69.0) / 12.0)
}

/// Clamps a pitch to the representable UltraStar range.
pub fn clamp_pitch(pitch: Pitch) -> Pitch {
    pitch.clamp(PITCH_MIN, PITCH_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concert_a_maps_to_pitch_nine() {
        assert_eq!(hz_to_pitch(440.0), 9);
        assert_eq!(pitch_to_midi(9), 69);
    }

    #[test]
    fn middle_c_maps_to_zero() {
        assert_eq!(hz_to_pitch(midi_to_hz(60.0)), 0);
    }

    #[test]
    fn range_clamping() {
        assert_eq!(clamp_pitch(-100), PITCH_MIN);
        assert_eq!(clamp_pitch(100), PITCH_MAX);
        assert_eq!(clamp_pitch(0), 0);
    }
}
