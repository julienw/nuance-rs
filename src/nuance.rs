use std::sync::mpsc;
use std::{ cmp, fmt, thread };
use std::io::{ Read, Result as IoResult, Error as IoError, ErrorKind as IoErrorKind };

use hyper::{ Client, Url };
use hyper::client::{ Request, Response };
use hyper::net::Streaming;
use hyper::header::{ ContentType, Accept, AcceptLanguage, Encoding, TransferEncoding, qitem };
use hyper::mime::{ Mime, TopLevel, SubLevel };
use hyper::LanguageTag;
use types::*;

const NUANCE_CONF_FILE: &'static str = "conf/nuance.ini";

#[derive(Debug, Clone)]
struct NuanceConfig {
    app_id: String,
    app_key: String,
    user_opaque_id: String,
    asr_uri: String,
    tts_uri: String,
    stt_uri: String,
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
            stt_uri: general_section.get("stt_uri").unwrap().to_string(),
        }
    }

    fn sanitize(mut self) -> Self {
        if !self.asr_uri.starts_with("https:") {
            self.asr_uri = "https://".to_string() + &self.asr_uri;
        }

        if !self.tts_uri.starts_with("https:") {
            self.tts_uri = "https://".to_string() + &self.tts_uri;
        }

        if !self.stt_uri.starts_with("https:") {
            self.stt_uri = "https://".to_string() + &self.stt_uri;
        }

        self
    }
}

pub struct TtsResponse {
    pub sound: Sound,
    pub frequency: Frequency
}

pub struct SttResponse {
    pub text_receiver: mpsc::Receiver<String>,
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

    pub fn with_bitrate_frequency(bitrate: Bitrate, frequency: Frequency) -> Nuance {
        Nuance {
            bitrate: bitrate,
            frequency: frequency,
            config: NuanceConfig::load().sanitize(),
        }
    }

    pub fn tts(&self, text: &str) -> TtsResponse {
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

    pub fn stt(&self, audio_receiver: mpsc::Receiver<Sound>, language: LanguageTag) -> SttResponse {
        let (text_sender, text_receiver) = mpsc::channel();

        let config = self.config.clone();
        let bitrate = self.bitrate;
        let frequency = self.frequency;
        let handle = thread::spawn(move || {
            let mut body = ReceiverBody::new(audio_receiver);
            let nuance_stt = NuanceStt::start_request(&config, language, bitrate, frequency, &mut body);
        });

        SttResponse {
            text_receiver: text_receiver
        }
    }
}

struct ReceiverBody {
    receiver: mpsc::Receiver<Sound>,
    current: Option<Vec<i8>>,
    counter: usize,
}

impl ReceiverBody {
    fn new(receiver: mpsc::Receiver<Sound>) -> ReceiverBody {
        ReceiverBody {
            receiver: receiver,
            current: None,
            counter: 0,
        }
    }

    fn clone_to_buffer(&mut self, dest: &mut [u8]) -> IoResult<usize> {
        let (written_length, source_length) = {
            let source = self.current.as_ref().unwrap();
            let counter = self.counter;
            let length = cmp::min(source.len() - counter, dest.len());
            for i in 0..length {
                dest[i] = source[counter + i] as u8;
            }
            self.counter = counter + length;
            (length, source.len())
        };
        if self.counter >= source_length {
            self.current = None;
            self.counter = 0;
        }
        Ok(written_length)
    }
}

impl Read for ReceiverBody {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if self.current.is_none() {
            let data = match self.receiver.recv() {
                Err(_) => return Ok(0),
                Ok(data) => data
            };
            match data {
                Sound::Bits_8(data) => self.current = Some(data),
                Sound::Bits_16(data) => {
                    return Err(IoError::new(IoErrorKind::InvalidData,
                                            "We do not support 16 bits payload yet."))
                }
            }
        }

        self.clone_to_buffer(buf)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DictationAudioSource {
    SpeakerAndMicrophone,
    HeadsetInOut,
    HeadsetBT,
    HeadPhone,
    LineOut,
}

impl fmt::Display for DictationAudioSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ::std::str::FromStr for DictationAudioSource {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SpeakerAndMicrophone" => Ok(DictationAudioSource::SpeakerAndMicrophone),
            "HeadsetInOut" => Ok(DictationAudioSource::HeadsetInOut),
            "HeadsetBT" => Ok(DictationAudioSource::HeadsetBT),
            "HeadPhone" => Ok(DictationAudioSource::HeadPhone),
            "LineOut" => Ok(DictationAudioSource::LineOut),
            _ => Err("Unknown DictationAudioSource")
        }
    }
}

header! { (XDictationAudioSource, "X-Dictation-AudioSource") => [DictationAudioSource] }

struct NuanceStt {
    response: Response,
}

impl NuanceStt {
    fn start_request<R: Read>(config: &NuanceConfig, language: LanguageTag, bitrate: Bitrate, frequency: Frequency, body: &mut R) -> NuanceStt {
        let client = Client::new();
        let mut url = Url::parse(&config.stt_uri).unwrap();
        url.query_pairs_mut()
            .append_pair("appId", &config.app_id)
            .append_pair("appKey", &config.app_key)
            .append_pair("id", &config.user_opaque_id);

        let audio_mime = format!("audio/x-wav;codec=pcm;bit={};rate={}",
                                 u8::from(bitrate), u32::from(frequency));
        let audio_mime: Mime = audio_mime.parse().unwrap();

        let mut res = client.post(url)
            .header(ContentType(audio_mime))
            .header(Accept(vec![qitem(Mime(TopLevel::Text, SubLevel::Plain, vec![]))]))
            .header(AcceptLanguage(vec![qitem(language)]))
            .header(TransferEncoding(vec![Encoding::Chunked]))
            .header(XDictationAudioSource(DictationAudioSource::SpeakerAndMicrophone))
            .body(body)
            .send().unwrap();

        info!("got result {:?}", res);

        NuanceStt {
            response: res
        }
    }
}

