use light_curve_common::linspace;
use light_curve_feature::*;
use light_curve_interpol::Interpolator;

#[cfg(feature = "hdf")]
use std::fs::File;
#[cfg(feature = "hdf")]
use std::io::Write;
#[cfg(feature = "hdf")]
use std::path::Path;

pub mod ch;
use ch::CHSourceDataBase;

pub mod config;
use config::{Config, DataBase};

mod dump;
use dump::Dumper;

#[cfg(feature = "hdf")]
mod hdf;
#[cfg(feature = "hdf")]
use hdf::Hdf5Cache;

mod lc;

mod traits;
#[cfg(feature = "hdf")]
use traits::Cache;
use traits::{ObservationsToSources, SourceDataBase};

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct TruncMedianNyquistFreq {
    m: MedianNyquistFreq,
    max_freq: f32,
}

#[allow(dead_code)]
impl TruncMedianNyquistFreq {
    fn new(min_dt: f32) -> Self {
        Self {
            m: MedianNyquistFreq,
            max_freq: std::f32::consts::PI / min_dt,
        }
    }
}

#[allow(dead_code)]
impl NyquistFreq<f32> for TruncMedianNyquistFreq {
    fn nyquist_freq(&self, t: &[f32]) -> f32 {
        f32::min(self.m.nyquist_freq(t), self.max_freq)
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct TruncQuantileNyquistFreq {
    q: QuantileNyquistFreq,
    max_freq: f32,
}

#[allow(dead_code)]
impl TruncQuantileNyquistFreq {
    fn new(quantile: f32, min_dt: f32) -> Self {
        Self {
            q: QuantileNyquistFreq { quantile },
            max_freq: std::f32::consts::PI / min_dt,
        }
    }
}

#[allow(dead_code)]
impl NyquistFreq<f32> for TruncQuantileNyquistFreq {
    fn nyquist_freq(&self, t: &[f32]) -> f32 {
        f32::min(self.q.nyquist_freq(t), self.max_freq)
    }
}

pub fn run(config: Config) {
    let mut dumper = Dumper::new(&config.passbands);

    if let Some(ref sid_path) = config.sid_path {
        dumper.set_sid_writer(sid_path.clone());
    }

    if let Some(ic) = &config.interpolation_config {
        let interpolator = Interpolator {
            target_x: linspace(58194.5_f32, 58482.5, 145),
            left: 0.,
            right: 0.,
        };
        dumper.set_interpolator(ic.path.clone(), interpolator);
    }

    if let Some(fc) = &config.feature_config {
        let mut periodogram_feature_evaluator = Periodogram::new(5);
        periodogram_feature_evaluator.set_nyquist(Box::new(AverageNyquistFreq));
        periodogram_feature_evaluator.set_freq_resolution(10.0);
        periodogram_feature_evaluator.set_max_freq_factor(2.0);
        periodogram_feature_evaluator.add_feature(Box::new(Amplitude::default()));
        periodogram_feature_evaluator.add_feature(Box::new(BeyondNStd::new(2.0)));
        periodogram_feature_evaluator.add_feature(Box::new(BeyondNStd::new(3.0)));
        periodogram_feature_evaluator.add_feature(Box::new(StandardDeviation::default()));

        let magn_feature_extractor = feat_extr!(
            Amplitude::default(),
            AndersonDarlingNormal::default(),
            BeyondNStd::new(1.0), // default
            BeyondNStd::new(2.0),
            Cusum::default(),
            EtaE::default(),
            InterPercentileRange::new(0.02),
            InterPercentileRange::new(0.1),
            InterPercentileRange::new(0.25),
            Kurtosis::default(),
            LinearFit::default(),
            LinearTrend::default(),
            MagnitudePercentageRatio::new(0.4, 0.05), // default
            MagnitudePercentageRatio::new(0.2, 0.05),
            MaximumSlope::default(),
            Mean::default(),
            MedianAbsoluteDeviation::default(),
            MedianBufferRangePercentage::new(0.1),
            MedianBufferRangePercentage::new(0.2),
            PercentAmplitude::default(),
            PercentDifferenceMagnitudePercentile::new(0.05), // default
            PercentDifferenceMagnitudePercentile::new(0.1),
            periodogram_feature_evaluator,
            ReducedChi2::default(),
            Skew::default(),
            StandardDeviation::default(),
            StetsonK::default(),
            WeightedMean::default(),
        );

        let flux_feature_extractor = feat_extr!(
            AndersonDarlingNormal::default(),
            Cusum::default(),
            EtaE::default(),
            ExcessVariance::default(),
            Kurtosis::default(),
            MeanVariance::default(),
            ReducedChi2::default(),
            Skew::default(),
            StetsonK::default(),
        );

        dumper.set_feature_extractor(
            fc.value_path.clone(),
            fc.name_path.clone(),
            magn_feature_extractor,
            flux_feature_extractor,
        );
    }

    dump_data(&mut dumper, &config);

    dumper.write_names();
}

#[cfg(feature = "hdf")]
fn dump_data(dumper: &mut Dumper, config: &Config) {
    let read_cache = match &config.cache_config {
        Some(cc) => {
            let cache = Box::new(Hdf5Cache {
                path: cc.data_path.clone(),
            });
            if Path::new(&cc.query_path).exists() {
                let query_cache = std::fs::read(&cc.query_path).unwrap();
                let query_from_file = String::from_utf8_lossy(&query_cache);
                assert_eq!(
                    query_from_file, config.sql_query,
                    "Cached SQL query mismatched specified one"
                );
                Some(cache)
            } else {
                let mut query_file = File::create(cc.query_path.clone()).unwrap();
                write!(query_file, "{}", config.sql_query).unwrap();
                dumper.set_write_cache(cache);
                None
            }
        }
        None => None,
    };

    match read_cache {
        Some(cache) => {
            dumper.dump_query_iter(cache.reader());
        }
        None => dump_from_db(dumper, config),
    }
}

#[cfg(not(feature = "hdf"))]
fn dump_data(dumper: &mut Dumper, config: &Config) {
    dump_from_db(dumper, config);
}

fn dump_from_db(dumper: &mut Dumper, config: &Config) {
    match config.database {
        DataBase::ClickHouse => {
            let mut source_db = CHSourceDataBase::new(&config.connection_config);
            let query = source_db.query(&config.sql_query);
            let source_iter = query.into_iter().sources(config.light_curves_are_sorted);
            dumper.dump_query_iter(source_iter);
        }
    }
}
