extern crate ini;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate alsa;

mod nuance;
mod types;

use nuance::Nuance;
use types::*;

fn play_sound(sound: &Sound, frequency: Frequency) {
    use std::ffi::CString;
    use alsa::{Direction, ValueOr};
    use alsa::pcm::{PCM, HwParams, Format, Access, State};

    // Open default playback device
    let pcm = PCM::open(&*CString::new("default").unwrap(), Direction::Playback, false).unwrap();

    // Set hardware parameters coming from parameters
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(1).unwrap();
    hwp.set_rate(u32::from(frequency), ValueOr::Nearest).unwrap();
    let format = match sound {
        &Sound::Bits_8(_) => Format::S8,
        &Sound::Bits_16(_) => Format::s16(),
    };
    hwp.set_format(format).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();
    match sound {
        &Sound::Bits_8(ref sound) => {
            let io = pcm.io_i8().unwrap();
            io.writei(sound).unwrap();
        }
        &Sound::Bits_16(ref sound) => {
            let io = pcm.io_i16().unwrap();
            io.writei(sound).unwrap();
        }
    }

    // In case the buffer was larger than 2 seconds, start the stream manually.
    if pcm.state() != State::Running { pcm.start().unwrap() };
    // Wait for the stream to finish playback.
    pcm.drain().unwrap();
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
