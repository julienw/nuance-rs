extern crate ini;
#[macro_use]
extern crate hyper;
#[macro_use]
extern crate log;
extern crate env_logger;

extern crate portaudio;
#[macro_use]
extern crate language_tags;

mod nuance;
mod types;

use std::sync::mpsc;
use std::time::{ Duration, Instant };
use std::{ io, thread };
use std::sync::Arc;
use std::sync::atomic::{ AtomicBool, Ordering };

use nuance::Nuance;
use types::*;

fn play_frames<T: portaudio::Sample + 'static>(frames: &[T], frequency: Frequency) -> Result<(), portaudio::Error> {
    let portaudio = try!(portaudio::PortAudio::new());

    let def_output = try!(portaudio.default_output_device());
    let output_info = try!(portaudio.device_info(def_output));
    println!("Default output device info: {:#?}", &output_info);

    // Construct the output stream parameters.
    let latency = output_info.default_high_output_latency;
    let output_params = portaudio::StreamParameters::<T>::new(def_output, /* channels */ 1, /* interleaved */ true, latency);

    // Check that the stream format is supported.
    try!(portaudio.is_output_format_supported(output_params, u32::from(frequency) as f64));
    let settings = portaudio::OutputStreamSettings::new(output_params, u32::from(frequency) as f64, 1024);

    let mut stream = try!(portaudio.open_blocking_stream(settings));
    try!(stream.start());

    let mut count: usize = 0;
    while count < frames.len() {
        let available = try!(stream.write_available());

        let available = match available {
            portaudio::StreamAvailable::Frames(available) => available as u32,
            portaudio::StreamAvailable::InputOverflowed => { println!("Input stream has overflowed"); continue }
            portaudio::StreamAvailable::OutputUnderflowed => { println!("Output stream has underflowed"); continue }
        };

        let will_write = std::cmp::min(available, (frames.len() - count) as u32);

        try!(stream.write(will_write, |output| {
            for i in 0..output.len() {
                output[i] = frames[count];
                count += 1;
            }
        }));
    }

    try!(stream.close());
    Ok(())
}

fn play_sound(sound: &Sound, frequency: Frequency) -> Result<(), portaudio::Error> {
    match sound {
        &Sound::Bits_8(ref frames) => play_frames(frames, frequency),
        &Sound::Bits_16(ref frames) => play_frames(frames, frequency),
    }
}

fn record_sound(bitrate: Bitrate, frequency: Frequency, should_stop: Arc<AtomicBool>, sender: mpsc::Sender<Sound>) -> Result<(), portaudio::Error> {
    match frequency {
        Frequency::Freq_8000 | Frequency::Freq_16000 => {}
        _ => panic!("Incorrect frequency was given, only 8k and 16k are supported.")
    }

    let portaudio = try!(portaudio::PortAudio::new());

    let def_input = try!(portaudio.default_input_device());
    let input_info = try!(portaudio.device_info(def_input));
    println!("Default input device info: {:#?}", &input_info);

    // Construct the input stream parameters.
    let latency = input_info.default_low_input_latency;
    let input_params = portaudio::StreamParameters::<u8>::new(def_input, /* channels */ 1, /* interleaved */ true, latency);

    // Check that the stream format is supported.
    try!(portaudio.is_input_format_supported(input_params, u32::from(frequency) as f64));
    let settings = portaudio::InputStreamSettings::new(input_params, u32::from(frequency) as f64, 256);

    let start_instant = Instant::now();

    let callback = move |portaudio::InputStreamCallbackArgs { buffer, .. }| {
        sender.send(Sound::from_vec_u8(buffer.to_vec()));

        if should_stop.load(Ordering::Relaxed) {
            println!("Ending recording.");
            portaudio::StreamCallbackResult::Complete
        } else {
            portaudio::StreamCallbackResult::Continue
        }
    };

    let mut stream = try!(portaudio.open_non_blocking_stream(settings, callback));

    try!(stream.start());

    while try!(stream.is_active()) {
        thread::sleep(Duration::from_secs(1));
    }

    try!(stream.stop());
    Ok(())
}

fn test_tts() {
    let nuance = Nuance::new();
    let result = nuance.tts("Salut aujourd'hui c'est l'été ! J'ai envie d'aller au cinéma, pas toi ?");
    play_sound(&result.sound, result.frequency).unwrap();
}

fn test_stt() {
    let mut input = String::new();
    loop {
        println!("Press enter to start recording.");

        // Note: read_line includes the newline character.
        io::stdin().read_line(&mut input).unwrap();
        input.clear(); // we don't really care about this

        let bitrate = Bitrate::Bits_8;
        let frequency = Frequency::Freq_8000;

        let (audio_sender, audio_receiver) = mpsc::channel();

        println!("Starting Nuance request...");
        let nuance = Nuance::with_bitrate_frequency(bitrate, frequency);
        let result = nuance.stt(audio_receiver, langtag!(eng;;;USA));
        println!("Recording sound...");

        println!("Press enter to stop recording.");
        let should_stop = Arc::new(AtomicBool::new(false));
        let cloned_should_stop = should_stop.clone();
        let recording_handle = thread::spawn(move || {
            record_sound(bitrate, frequency, cloned_should_stop, audio_sender).unwrap();
        });

        io::stdin().read_line(&mut input).unwrap();
        input.clear(); // we don't really care about this
        should_stop.store(true, Ordering::Relaxed);
        recording_handle.join().unwrap();
    }
}

fn main() {
    env_logger::init().unwrap();

    test_stt();
}
