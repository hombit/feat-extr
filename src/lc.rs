use hdf5::H5Type;
use light_curve_common::sort_multiple;
use std::fmt;

pub const MJD0: f64 = 58000.0;

#[derive(H5Type, Copy, Clone)]
#[repr(u8)]
pub enum Passband {
    G = 1,
    R = 2,
    I = 3,
}

impl Passband {
    fn lcs_index(&self) -> usize {
        match self {
            Self::G => 0,
            Self::R => 1,
            Self::I => 2,
        }
    }

    pub fn from_lcs_index(i: usize) -> Self {
        match i {
            0 => Self::G,
            1 => Self::R,
            2 => Self::I,
            _ => panic!("unknown lcs index {}", i),
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            Self::G => 1,
            Self::R => 2,
            Self::I => 3,
        }
    }

    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::G,
            2 => Self::R,
            3 => Self::I,
            _ => panic!("unknown filter code {}", code),
        }
    }
}

impl fmt::Display for Passband {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Passband::G => "g",
            Passband::R => "r",
            Passband::I => "i",
        };
        write!(f, "{}", s)
    }
}

impl From<String> for Passband {
    fn from(item: String) -> Self {
        match item.as_str() {
            "g" => Passband::G,
            "r" => Passband::R,
            "i" => Passband::I,
            _ => panic!("passband {} is unknown", item),
        }
    }
}

#[derive(Clone)]
pub struct Source {
    pub sid: u64,
    pub lcs: [LightCurve; 3],
}

impl Source {
    pub fn lc(&self, passband: Passband) -> &LightCurve {
        &self.lcs[passband.lcs_index()]
    }

    pub fn lc_mut(&mut self, passband: Passband) -> &mut LightCurve {
        &mut self.lcs[passband.lcs_index()]
    }

    pub fn push_observation(&mut self, obs: &Observation) {
        self.lc_mut(obs.passband).push_observation(obs)
    }

    pub fn sort(&mut self) {
        for lc in self.lcs.iter_mut() {
            lc.sort();
        }
    }

    pub fn len(&self) -> usize {
        self.lcs.iter().map(|lc| lc.t.len()).sum()
    }

    pub fn iter_observations(&self) -> impl Iterator<Item = Observation> + '_ {
        self.lcs.iter().enumerate().flat_map(move |(i, lc)| {
            let passband = Passband::from_lcs_index(i);
            lc.t.iter()
                .zip(lc.mag.iter())
                .zip(lc.w.iter())
                .map(move |((&t, &mag), &w)| Observation {
                    sid: self.sid,
                    t,
                    mag,
                    w,
                    passband,
                })
        })
    }
}

#[derive(Clone)]
pub struct LightCurve {
    pub t: Vec<f32>,
    pub mag: Vec<f32>,
    pub w: Vec<f32>,
}

impl LightCurve {
    pub fn push_observation(&mut self, obs: &Observation) {
        self.t.push(obs.t);
        self.mag.push(obs.mag);
        self.w.push(obs.w);
    }

    pub fn sort(&mut self) {
        let mut tmw = sort_multiple(&[&self.t, &self.mag, &self.w]);
        self.w = tmw.pop().unwrap();
        self.mag = tmw.pop().unwrap();
        self.t = tmw.pop().unwrap();
        assert!(tmw.is_empty());
    }
}

#[derive(H5Type, Clone)]
#[repr(C)]
pub struct Observation {
    pub sid: u64,
    pub t: f32,
    pub mag: f32,
    pub w: f32,
    pub passband: Passband,
}
