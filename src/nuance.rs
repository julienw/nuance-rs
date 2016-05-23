use types::*;

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
    fn load() -> Self {
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

pub struct TtsResponse {
    pub sound: Sound,
    pub frequency: Frequency
}

pub struct Nuance {
    bitrate: Bitrate,
    frequency: Frequency,
    config: NuanceConfig,
}

impl Nuance {
    pub fn new() -> Nuance {
        Nuance {
            bitrate: Bitrate::Bits_16,
            frequency: Frequency::Freq_22000,
            config: NuanceConfig::load().sanitize(),
        }
    }

    pub fn tts(&self, text: &str) -> TtsResponse {
        use hyper::{ Client, Url };
        use hyper::header::{ ContentType, Accept, qitem };
        use hyper::mime::Mime;
        use std::io::Read;

        let client = Client::new();
        let mut url = Url::parse(&self.config.tts_uri).unwrap();
        url.query_pairs_mut()
            .append_pair("appId", &self.config.app_id)
            .append_pair("appKey", &self.config.app_key)
            .append_pair("id", &self.config.user_opaque_id)
            .append_pair("voice", "Aurelie");

        let audio_mime = format!("audio/x-wav;codec=pcm;bit={};rate={}",
                                 u8::from(self.bitrate), u32::from(self.frequency));
        let audio_mime: Mime = audio_mime.parse().unwrap();

        let mut res = client.post(url)
            .header(ContentType::plaintext())
            .header(Accept(vec![qitem(audio_mime)]))
            .body(text)
            .send().unwrap();

        info!("got result {:?}", res);

        let (bitrate, frequency) = {
            let return_type = res.headers.get::<ContentType>().unwrap();
            let mime = &return_type.0;
            // TODO no unwrap for what comes from Internet
            let bitrate = Bitrate::from_u8(mime.get_param("bit").unwrap().parse().unwrap()).unwrap();
            let frequency = Frequency::from_u32(mime.get_param("rate").unwrap().parse().unwrap()).unwrap();
            (bitrate, frequency)
        };

        let mut body: Vec<u8> = Vec::new();
        res.read_to_end(&mut body).unwrap();

        let body = match bitrate {
            Bitrate::Bits_8 => {
                let body_8bits: Vec<i8> = body.drain(..).map(|data| data as i8).collect();
                Sound::Bits_8(body_8bits)
            }
            Bitrate::Bits_16 => {
                let body_16bits: Vec<i16> = body.chunks(2).map(|data| {
                    ((data[0] as i16) << 0) | ((data[1] as i16) << 8)
                }).collect();
                Sound::Bits_16(body_16bits)
            }
        };

        TtsResponse {
            sound: body,
            frequency: frequency,
        }
    }
}

