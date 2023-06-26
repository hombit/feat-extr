use light_curve_feature::*;
use std::str::FromStr;

pub enum FeatureVersion {
    Snad4,
    Snad6,
}

impl FromStr for FeatureVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "snad4" => Ok(Self::Snad4),
            "snad6" => Ok(Self::Snad6),
            _ => Err(format!("unknown feature version: {}", s)),
        }
    }
}

impl FeatureVersion {
    fn snad4_magn_extractor() -> Feature<f32> {
        let mut periodogram_feature_evaluator = Periodogram::new(5);
        periodogram_feature_evaluator.set_nyquist(NyquistFreq::fixed(24.0));
        periodogram_feature_evaluator.set_freq_resolution(10.0);
        periodogram_feature_evaluator.set_max_freq_factor(2.0);
        periodogram_feature_evaluator.add_feature(Amplitude::default().into());
        periodogram_feature_evaluator.add_feature(BeyondNStd::new(2.0).into());
        periodogram_feature_evaluator.add_feature(BeyondNStd::new(3.0).into());
        periodogram_feature_evaluator.add_feature(StandardDeviation::default().into());

        FeatureExtractor::from_features(vec![
            Amplitude::default().into(),
            AndersonDarlingNormal::default().into(),
            BeyondNStd::new(1.0).into(), // default
            BeyondNStd::new(2.0).into(),
            Cusum::default().into(),
            EtaE::default().into(),
            InterPercentileRange::new(0.02).into(),
            InterPercentileRange::new(0.1).into(),
            InterPercentileRange::new(0.25).into(),
            Kurtosis::default().into(),
            LinearFit::default().into(),
            LinearTrend::default().into(),
            MagnitudePercentageRatio::new(0.4, 0.05).into(), // default
            MagnitudePercentageRatio::new(0.2, 0.05).into(),
            MaximumSlope::default().into(),
            Mean::default().into(),
            MedianAbsoluteDeviation::default().into(),
            MedianBufferRangePercentage::new(0.1).into(),
            MedianBufferRangePercentage::new(0.2).into(),
            PercentAmplitude::default().into(),
            PercentDifferenceMagnitudePercentile::new(0.05).into(), // default
            PercentDifferenceMagnitudePercentile::new(0.1).into(),
            periodogram_feature_evaluator.into(),
            ReducedChi2::default().into(),
            Skew::default().into(),
            StandardDeviation::default().into(),
            StetsonK::default().into(),
            WeightedMean::default().into(),
        ])
        .into()
    }

    fn snad6_magn_extractor() -> Feature<f32> {
        FeatureExtractor::from_features(vec![InterPercentileRange::new(0.02).into()]).into()
    }

    pub fn magn_extractor(&self) -> Feature<f32> {
        match self {
            Self::Snad4 => Self::snad4_magn_extractor(),
            Self::Snad6 => Self::snad6_magn_extractor(),
        }
    }

    fn snad4_flux_extractor() -> Feature<f32> {
        FeatureExtractor::from_features(vec![
            AndersonDarlingNormal::default().into(),
            Cusum::default().into(),
            EtaE::default().into(),
            ExcessVariance::default().into(),
            Kurtosis::default().into(),
            MeanVariance::default().into(),
            ReducedChi2::default().into(),
            Skew::default().into(),
            StetsonK::default().into(),
        ])
        .into()
    }

    fn snad6_flux_extractor() -> Feature<f32> {
        FeatureExtractor::from_features(vec![AndersonDarlingNormal::default().into()]).into()
    }

    pub fn flux_extractor(&self) -> Feature<f32> {
        match self {
            Self::Snad4 => Self::snad4_flux_extractor(),
            Self::Snad6 => Self::snad6_flux_extractor(),
        }
    }
}
