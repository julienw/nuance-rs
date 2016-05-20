extern crate ini;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate alsa;

use std::io::{ Read, Write };

const NUANCE_CONF_FILE: &'static str = "conf/nuance.ini";

#[derive(Debug)]
struct NuanceConfig {
    app_id: String,
    app_key: String,
    user_opaque_id: String,
    asr_uri: String,
    tts_uri: String,
}

impl NuanceConfig {
    fn sanitize(mut self) -> Self {
        if !self.asr_uri.starts_with("https:") {
            self.asr_uri = "https://".to_string() + &self.asr_uri;
        }

        if !self.tts_uri.starts_with("https:") {
            self.tts_uri = "https://".to_string() + &self.tts_uri;
        }

        self
    }
}

fn read_conf() -> NuanceConfig {
    use ini::Ini;
    let conf = Ini::load_from_file(NUANCE_CONF_FILE).unwrap();
    let general_section = conf.general_section();

    NuanceConfig {
        app_id: general_section.get("app_id").unwrap().to_string(),
        app_key: general_section.get("app_key").unwrap().to_string(),
        user_opaque_id: general_section.get("user_opaque_id").unwrap().to_string(),
        asr_uri: general_section.get("asr_uri").unwrap().to_string(),
        tts_uri: general_section.get("tts_uri").unwrap().to_string(),
    }
}

fn play_sound(sound: &[u8]) {
    use std::ffi::CString;
    use alsa::{Direction, ValueOr};
    use alsa::pcm::{PCM, HwParams, Format, Access, State};

    // Open default playback device
    let pcm = PCM::open(&*CString::new("default").unwrap(), Direction::Playback, false).unwrap();

    // Set hardware parameters: 8 kHz / Mono / 16 bit
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(1).unwrap();
    hwp.set_rate(8000, ValueOr::Nearest).unwrap();
    hwp.set_format(Format::s16()).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();
    let mut io = pcm.io_i16().unwrap();

    // Make a sine wave
    let mut buf = [0i16; 1024];
    for (i, a) in buf.iter_mut().enumerate() {
        *a = ((i as f32 * 2.0 * ::std::f32::consts::PI / 128.0).sin() * 8192.0) as i16
    }

    //io.writei(sound).unwrap();
    io.write_all(sound).unwrap();

    // In case the buffer was larger than 2 seconds, start the stream manually.
    if pcm.state() != State::Running { pcm.start().unwrap() };
    // Wait for the stream to finish playback.
    pcm.drain().unwrap();
}

fn main() {
    use hyper::{ Client, Url };
    use hyper::header::{ ContentType, Accept, qitem };
    use hyper::mime::Mime;

    env_logger::init().unwrap();

    let conf = read_conf().sanitize();

    info!("found config {:?}", conf);

    let client = Client::new();
    let mut url = Url::parse(&conf.tts_uri).unwrap();
    url.query_pairs_mut()
        .append_pair("appId", &conf.app_id)
        .append_pair("appKey", &conf.app_key)
        .append_pair("id", &conf.user_opaque_id)
        .append_pair("voice", "Amelie");

    let audio_mime: Mime = "audio/x-wav;codec=pcm;bit=16;rate=8000".parse().unwrap();

    let mut res = client.post(url)
        .header(ContentType::plaintext())
        .header(Accept(vec![qitem(audio_mime)]))
        .body("Salut aujourd'hui c'est l'été ! J'ai envie d'aller au cinéma, pas toi ?")
        .send().unwrap();

    info!("got result {:?}", res);

    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();
    play_sound(&body);
}
