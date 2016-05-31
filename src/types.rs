#[derive(Debug, Copy, Clone)]
pub enum Bitrate {
    Bits_8,
    Bits_16,
}

impl From<Bitrate> for u8 {
    fn from(bitrate: Bitrate) -> Self {
        match bitrate {
            Bitrate::Bits_8 => 8,
            Bitrate::Bits_16 => 16,
        }
    }
}

impl Bitrate {
    pub fn from_u8(bitrate: u8) -> Option<Bitrate> {
        match bitrate {
            8 => Some(Bitrate::Bits_8),
            16 => Some(Bitrate::Bits_16),
            _ => None
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Frequency {
    Freq_8000,
    Freq_16000,
    Freq_22000,
    Freq_44100,
}

impl From<Frequency> for u32 {
    fn from(frequency: Frequency) -> Self {
        match frequency {
            Frequency::Freq_8000 => 8000,
            Frequency::Freq_16000 => 16000,
            Frequency::Freq_22000 => 22000,
            Frequency::Freq_44100 => 44100,
        }
    }
}

impl Frequency {
    pub fn from_u32(frequency: u32) -> Option<Frequency> {
        match frequency {
            8000 => Some(Frequency::Freq_8000),
            16000 => Some(Frequency::Freq_16000),
            22000 => Some(Frequency::Freq_22000),
            44100 =>  Some(Frequency::Freq_44100),
            _ => None
        }
    }
}

pub enum Sound {
    Bits_8(Vec<i8>),
    Bits_16(Vec<i16>),
}

impl Sound {
    pub fn from_vec_u8(mut vec: Vec<u8>) -> Sound {
        let vec_i8 = vec.drain(..).map(|frame| frame as i8).collect();
        Sound::Bits_8(vec_i8)
    }

    pub fn from_vec_i16(vec: Vec<i16>) -> Sound {
        Sound::Bits_16(vec)
    }
}
