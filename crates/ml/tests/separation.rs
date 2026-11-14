use std::path::PathBuf;
use std::sync::Arc;

use charon_audio::{AudioBuffer, AudioFile};
use ndarray::Array2;
use usdx_ml::{CharonSeparator, VocalEnergy};
use usdx_ports::{CancelFlag, SeparationControl, StemSeparator};

fn synthetic_mix(sample_rate: u32, seconds: u32) -> Array2<f32> {
    let samples = (sample_rate * seconds) as usize;
    let mut data = Array2::<f32>::zeros((2, samples));
    for index in 0..samples {
        let time = index as f32 / sample_rate as f32;
        let vocal = (2.0 * std::f32::consts::PI * 440.0 * time).sin() * 0.25;
        let instrument = (2.0 * std::f32::consts::PI * 220.0 * time).sin() * 0.25
            + (2.0 * std::f32::consts::PI * 330.0 * time).sin() * 0.1;
        let sample = vocal + instrument;
        data[[0, index]] = sample;
        data[[1, index]] = sample;
    }
    data
}

#[test]
fn separates_a_synthetic_mix_with_the_real_model() {
    let Some(model) = std::env::var_os("USDX_HTDEMUCS_MODEL").map(PathBuf::from) else {
        eprintln!("USDX_HTDEMUCS_MODEL is not set; skipping real separation test");
        return;
    };

    let directory = tempfile::tempdir().unwrap();
    let sample_rate = 44_100;
    let seconds = 6;
    let mix = synthetic_mix(sample_rate, seconds);
    let samples = mix.ncols();
    let input = directory.path().join("mix.wav");
    AudioFile::write_wav(&input, &AudioBuffer::new(mix.clone(), sample_rate)).unwrap();

    let separator = CharonSeparator::new(&model).unwrap();
    let control = SeparationControl::new(CancelFlag::default(), Arc::new(|_| {}));
    let first_dir = directory.path().join("first");
    let second_dir = directory.path().join("second");
    let paths = separator
        .separate(&input, &first_dir, control.clone())
        .unwrap();
    let paths_again = separator.separate(&input, &second_dir, control).unwrap();

    assert_eq!(
        std::fs::read(&paths.vocals).unwrap(),
        std::fs::read(&paths_again.vocals).unwrap(),
        "vocals stem is not deterministic"
    );
    assert_eq!(
        std::fs::read(&paths.instrumental).unwrap(),
        std::fs::read(&paths_again.instrumental).unwrap(),
        "instrumental stem is not deterministic"
    );

    let vocals = AudioFile::read(&paths.vocals).unwrap();
    let instrumental = AudioFile::read(&paths.instrumental).unwrap();
    assert_eq!(vocals.sample_rate, sample_rate);
    assert_eq!(instrumental.sample_rate, sample_rate);
    assert_eq!(vocals.samples(), samples);
    assert_eq!(instrumental.samples(), samples);
    assert_eq!(vocals.channels(), 2);
    assert_eq!(instrumental.channels(), 2);

    let mut difference = 0.0_f64;
    let mut original = 0.0_f64;
    for channel in 0..2 {
        for index in 0..samples {
            let reconstructed = vocals.data[[channel, index]] + instrumental.data[[channel, index]];
            let expected = mix[[channel, index]];
            difference += ((reconstructed - expected) as f64).powi(2);
            original += (expected as f64).powi(2);
        }
    }
    let relative_error = (difference / original.max(f64::EPSILON)).sqrt();
    assert!(
        relative_error < 0.25,
        "stem reconstruction error is too high: {relative_error}"
    );

    let energy = usdx_ml::analyze_vocal_energy(&paths.vocals).unwrap();
    assert!(energy.peak.is_finite());
    assert!(energy.rms.is_finite());
    assert!(matches!(energy, VocalEnergy { .. }));
}
