extern crate ini;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate env_logger;

extern crate portaudio;

mod nuance;
mod types;

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

/*
fn record_sound(bitrate: Bitrate, frequency: Frequency, Sender<&[u8]>) {

}
*/

fn main() {
    env_logger::init().unwrap();

    let nuance = Nuance::new();
    let result = nuance.tts("Salut aujourd'hui c'est l'été ! J'ai envie d'aller au cinéma, pas toi ?");
    play_sound(&result.sound, result.frequency);
}
