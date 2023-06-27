use light_curve_feature::transformers::{
    arcsinh::ArcsinhTransformer, bazin_fit::BazinFitTransformer, composed::ComposedTransformer,
    identity::IdentityTransformer, lg::LgTransformer, ln1p::Ln1pTransformer,
};
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
        let bins: Feature<f32> = {
            let eta_e: Feature<f32> = {
                let feature: Feature<f32> = EtaE::default().into();
                let transformer: Transformer<f32> = Ln1pTransformer {}.into();
                Transformed::new(feature, transformer).unwrap().into()
            };
            let linear_fit: Feature<f32> = {
                let feature: Feature<f32> = LinearFit::default().into();
                let transformer: Transformer<f32> = ComposedTransformer::new(vec![
                    (ArcsinhTransformer {}.into(), 1), // slope
                    (LgTransformer {}.into(), 1),      // slope sigma
                    (Ln1pTransformer {}.into(), 1),    // reduced chi2
                ])
                .unwrap()
                .into();
                Transformed::new(feature, transformer).unwrap().into()
            };
            let linear_trend: Feature<f32> = {
                let feature: Feature<f32> = LinearTrend::default().into();
                let transformer: Transformer<f32> = ComposedTransformer::new(vec![
                    (ArcsinhTransformer {}.into(), 1), // trend
                    (LgTransformer {}.into(), 1),      // trend sigma
                    (LgTransformer {}.into(), 1),      // noise
                ])
                .unwrap()
                .into();
                Transformed::new(feature, transformer).unwrap().into()
            };
            let maximum_slope: Feature<f32> = {
                let feature: Feature<f32> = MaximumSlope::default().into();
                let transformer: Transformer<f32> = LgTransformer {}.into();
                Transformed::new(feature, transformer).unwrap().into()
            };

            let mut bins = Bins::new(1.0, 0.0);
            bins.add_feature(Cusum::new().into());
            bins.add_feature(eta_e);
            bins.add_feature(linear_fit);
            bins.add_feature(linear_trend);
            bins.add_feature(maximum_slope);

            bins.into()
        };

        let inter_percentile_range_02: Feature<f32> = {
            let feature = InterPercentileRange::new(0.02).into();
            let transformer: Transformer<f32> = LgTransformer {}.into();
            Transformed::new(feature, transformer).unwrap().into()
        };
        let inter_percentile_range_10: Feature<f32> = {
            let feature = InterPercentileRange::new(0.1).into();
            let transformer: Transformer<f32> = LgTransformer {}.into();
            Transformed::new(feature, transformer).unwrap().into()
        };
        let inter_percentile_range_25: Feature<f32> = {
            let feature = InterPercentileRange::new(0.25).into();
            let transformer: Transformer<f32> = LgTransformer {}.into();
            Transformed::new(feature, transformer).unwrap().into()
        };

        let periodogram: Feature<f32> = {
            let mut periodogram = Periodogram::new(1);
            periodogram.set_nyquist(NyquistFreq::fixed(24.0));
            periodogram.set_freq_resolution(10.0);
            periodogram.set_max_freq_factor(2.0);
            periodogram.into()
        };

        let otsu_split: Feature<f32> = {
            let feature = OtsuSplit::new().into();
            let transformer: Transformer<f32> = ComposedTransformer::new(vec![
                (LgTransformer {}.into(), 1),       // mean diff
                (LgTransformer {}.into(), 1),       // std lower
                (LgTransformer {}.into(), 1),       // std upper
                (IdentityTransformer {}.into(), 1), // lower to all ratio
            ])
            .unwrap()
            .into();
            Transformed::new(feature, transformer).unwrap().into()
        };

        let reduced_chi2: Feature<f32> = {
            let feature = ReducedChi2::new().into();
            let transformer: Transformer<f32> = Ln1pTransformer {}.into();
            Transformed::new(feature, transformer).unwrap().into()
        };

        let skew: Feature<f32> = {
            let feature = Skew::new().into();
            let transformer: Transformer<f32> = ArcsinhTransformer {}.into();
            Transformed::new(feature, transformer).unwrap().into()
        };

        FeatureExtractor::from_features(vec![
            BeyondNStd::new(1.0).into(), // default
            BeyondNStd::new(2.0).into(),
            bins,
            inter_percentile_range_02,
            inter_percentile_range_25,
            inter_percentile_range_10,
            Kurtosis::new().into(),
            otsu_split,
            periodogram.into(),
            reduced_chi2,
            skew,
            StetsonK::new().into(),
            WeightedMean::new().into(),
        ])
        .into()
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
        let anderson_darling_normal: Feature<f32> = {
            let feature = AndersonDarlingNormal::default().into();
            let transformer: Transformer<f32> = Ln1pTransformer {}.into();
            Transformed::new(feature, transformer).unwrap().into()
        };

        let bazin_fit: Feature<f32> = {
            let inits_bounds = BazinInitsBounds::option_arrays(
                [None; 5],
                [
                    Some(f64::powf(10.0, -0.4 * 30.0)), // amplitude
                    None,                               // baseline
                    None,                               // t0
                    Some(1e-4),                         // rise time
                    Some(1e-4),                         // fall time
                ],
                [
                    Some(f64::powf(10.0, -0.4 * 0.0)), // amplitude
                    None,                              // baseline
                    None,                              // t0
                    Some(1e5),                         // rise time
                    Some(1e5),                         // fall time
                ],
            );

            let fit = BazinFit::new(
                CeresCurveFit::new(20, None).into(),
                LnPrior::none(),
                inits_bounds,
            );
            let feature: Feature<f32> = fit.into();
            let transformer = Transformer::BazinFit(BazinFitTransformer::new(0.0));
            let transformed = Transformed::new(feature, transformer).unwrap();
            transformed.into()
        };

        FeatureExtractor::from_features(vec![
            anderson_darling_normal,
            bazin_fit,
            ExcessVariance::new().into(),
        ])
        .into()
    }

    pub fn flux_extractor(&self) -> Feature<f32> {
        match self {
            Self::Snad4 => Self::snad4_flux_extractor(),
            Self::Snad6 => Self::snad6_flux_extractor(),
        }
    }
}
