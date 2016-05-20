extern crate ini;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate portaudio;

use portaudio as pa;

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

fn play_sound() -> Result<(), pa::Error> {
    const SAMPLE_RATE: f64 = 44_100.0;
    const FRAMES: u32 = 256;
    const CHANNELS: i32 = 2;
    const INTERLEAVED: bool = true;

    let pa = try!(pa::PortAudio::new());

    let def_output = try!(pa.default_output_device());
    let output_info = try!(pa.device_info(def_output));
    info!("Default output device info: {:#?}", &output_info);

    /*
    // Construct the output stream parameters.
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::new(def_output, CHANNELS, INTERLEAVED, latency);
    */
    Ok(())
}

fn main() {
    use hyper::{ Client, Url };
    use hyper::header;

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

    info!("Will use the url {}", url);

    let res = client.post(url)
        .header(header::ContentType::plaintext())
        .body("Salut aujourd'hui c'est l'été !")
        .send().unwrap();

    info!("got result {:?}", res);
}
