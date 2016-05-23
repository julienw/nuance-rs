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

fn play_sound(sound: &Sound, frequency: Frequency) -> Result<(), portaudio::Error> {
    let portaudio = try!(portaudio::PortAudio::new());

    let def_output = try!(portaudio.default_output_device());
    let output_info = try!(portaudio.device_info(def_output));
    println!("Default output device info: {:#?}", &output_info);

    // Construct the output stream parameters.
    let latency = output_info.default_high_output_latency;
    let output_params = match sound {
        &Sound::Bits_8(_) => portaudio::StreamParameters::<u8>::new(def_output, /* channels */ 1, /* interleaved */ true, latency),
        &Sound::Bits_16(_) => portaudio::StreamParameters::<u16>::new(def_output, /* channels */ 1, /* interleaved */ true, latency),
    };

    // Check that the stream format is supported.
    try!(portaudio.is_output_format_supported(output_params, u32::from(frequency) as f64));
    let settings = portaudio::OutputStreamSettings::new(output_params, u32::from(frequency) as f64, 1024);

    let mut stream = try!(portaudio.open_blocking_stream(settings));
    try!(stream.start());

    let mut count: usize = 0;
    while count < sound.len() {
        let available = try!(stream.write_available());

        let available = match available {
            portaudio::StreamAvailable::Frames(frames) => frames as u32,
            portaudio::StreamAvailable::InputOverflowed => { println!("Input stream has overflowed"); continue }
            portaudio::StreamAvailable::OutputUnderflowed => { println!("Output stream has underflowed"); continue }
        };

        let will_write = std::cmp::min(available, (sound.len() - count) as u32);

        println!("1 Count is {}", count);
        try!(stream.write(will_write, |output| {
            println!("Will write {}", will_write);
            for i in 0..output.len() {
                output[i] = sound[count];
                count += 1;
            }
            println!("Wrote {}", will_write);
        }));
        println!("2 Count is {}", count);
    }

    try!(stream.close());
    Ok(())
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
