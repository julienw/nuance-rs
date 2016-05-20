extern crate ini;

const NUANCE_CONF_FILE: &'static str = "conf/nuance.ini";

#[derive(Debug)]
struct NuanceConfig {
    app_id: String,
    app_key: String,
    user_id: String,
    asr_uri: String,
    tts_uri: String,
}

fn read_conf() -> NuanceConfig {
    use ini::Ini;
    let conf = Ini::load_from_file(NUANCE_CONF_FILE).unwrap();
    let general_section = conf.general_section();
    NuanceConfig {
        app_id: general_section.get("app_id").unwrap().to_string(),
        app_key: general_section.get("app_key").unwrap().to_string(),
        user_id: general_section.get("user_id").unwrap().to_string(),
        asr_uri: general_section.get("asr_uri").unwrap().to_string(),
        tts_uri: general_section.get("tts_uri").unwrap().to_string(),
    }
}
fn main() {
    let config = read_conf();
    println!("found config {:?}", config);
}
