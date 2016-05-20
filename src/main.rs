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

use std::ops::Deref;

#[derive(Debug, Copy, Clone)]
struct Bitrate(u32);
#[derive(Debug, Copy, Clone)]
struct Frequency(u32);

impl Deref for Bitrate {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}

impl Deref for Frequency {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
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

fn play_sound(sound: &[u8], bitrate: Bitrate, frequency: Frequency) {
    use std::ffi::CString;
    use alsa::{Direction, ValueOr};
    use alsa::pcm::{PCM, HwParams, Format, Access, State};

    // Open default playback device
    let pcm = PCM::open(&*CString::new("default").unwrap(), Direction::Playback, false).unwrap();

    // Set hardware parameters coming from parameters
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(1).unwrap();
    hwp.set_rate(*frequency, ValueOr::Nearest).unwrap();
    let format = match bitrate {
        Bitrate(16) => Format::s16(),
        Bitrate(8) => Format::S8,
        _ => Format::Unknown,
    };
    hwp.set_format(format).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();
    let mut io = pcm.io_i16().unwrap();

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
        .append_pair("voice", "Aurelie");

    let mut bitrate = Bitrate(16);
    let mut frequency = Frequency(22000);
    let audio_mime = format!("audio/x-wav;codec=pcm;bit={};rate={}", &*bitrate, &*frequency);
    let audio_mime: Mime = audio_mime.parse().unwrap();

    let mut res = client.post(url)
        .header(ContentType::plaintext())
        .header(Accept(vec![qitem(audio_mime)]))
        .body("Salut aujourd'hui c'est l'été ! J'ai envie d'aller au cinéma, pas toi ?")
        .send().unwrap();

    info!("got result {:?}", res);

    {
        let return_type = res.headers.get::<ContentType>().unwrap();
        let mime = &return_type.0;
        bitrate = Bitrate(mime.get_param("bit").unwrap().parse().unwrap());
        frequency = Frequency(mime.get_param("rate").unwrap().parse().unwrap());
    }

    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();
    play_sound(&body, bitrate, frequency);
}
