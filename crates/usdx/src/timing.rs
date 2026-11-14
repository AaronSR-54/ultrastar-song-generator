use usdx_domain::SongHeader;

pub use usdx_domain::{
    clamp_pitch, hz_to_midi, hz_to_pitch, midi_to_hz, pitch_to_midi, PITCH_MAX, PITCH_MIN,
};

/// Converts an UltraStar beat position into milliseconds.
///
/// `#BPM` is four times the musical tempo, so one beat lasts
/// `60_000 / (bpm * 4)` milliseconds.
pub fn beat_to_ms(header: &SongHeader, beat: f64) -> f64 {
    if header.bpm <= 0.0 {
        return f64::NAN;
    }
    header.gap_ms as f64 + beat * 60_000.0 / (header.bpm * 4.0)
}

/// Converts milliseconds into a fractional UltraStar beat position.
pub fn ms_to_beat(header: &SongHeader, ms: f64) -> f64 {
    if header.bpm <= 0.0 {
        return f64::NAN;
    }
    (ms - header.gap_ms as f64) * header.bpm * 4.0 / 60_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(bpm: f64, gap_ms: i64) -> SongHeader {
        SongHeader {
            bpm,
            gap_ms,
            ..SongHeader::default()
        }
    }

    #[test]
    fn adele_hello_reference_timing() {
        let header = header(313.34, 81_090);
        let last_note_ms = beat_to_ms(&header, 5723.0);
        assert!((last_note_ms - 355_070.0).abs() < 1_000.0, "{last_note_ms}");
    }

    #[test]
    fn take_on_me_reference_timing() {
        let header = header(260.0, 14_096);
        let last_note_ms = beat_to_ms(&header, 3382.0);
        assert!((last_note_ms - 209_211.0).abs() < 1_000.0, "{last_note_ms}");
    }

    #[test]
    fn beat_zero_maps_to_gap() {
        let header = header(120.0, 12_345);
        assert!((beat_to_ms(&header, 0.0) - 12_345.0).abs() < f64::EPSILON);
    }

    #[test]
    fn beat_and_ms_are_inverse() {
        let header = header(313.34, 81_090);
        for beat in [0.0, 1.0, 55.0, 5723.0, 9999.5] {
            let ms = beat_to_ms(&header, beat);
            assert!((ms_to_beat(&header, ms) - beat).abs() < 1e-9);
        }
    }

    #[test]
    fn invalid_bpm_produces_nan() {
        let header = header(0.0, 0);
        assert!(beat_to_ms(&header, 10.0).is_nan());
        assert!(ms_to_beat(&header, 10.0).is_nan());
    }

    #[test]
    fn concert_a_maps_to_pitch_nine() {
        assert!((hz_to_midi(440.0) - 69.0).abs() < 1e-9);
        assert_eq!(hz_to_pitch(440.0), 9);
        assert_eq!(pitch_to_midi(9), 69);
    }

    #[test]
    fn middle_c_maps_to_pitch_zero() {
        let c4 = midi_to_hz(60.0);
        assert!((c4 - 261.625_565).abs() < 1e-5);
        assert_eq!(hz_to_pitch(c4), 0);
    }

    #[test]
    fn lower_and_higher_octaves() {
        assert_eq!(hz_to_pitch(220.0), -3);
        assert_eq!(hz_to_pitch(880.0), 21);
        assert!((midi_to_hz(69.0) - 440.0).abs() < 1e-9);
    }

    #[test]
    fn midi_round_trip_is_stable() {
        for midi in 36..=96 {
            let hz = midi_to_hz(midi as f64);
            assert_eq!(hz_to_midi(hz).round() as i16, midi);
        }
    }
}
