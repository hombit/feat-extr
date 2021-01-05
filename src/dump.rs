use crate::lc::{Passband, Source};
use crate::traits::*;
use crossbeam::channel::{bounded as bounded_channel, Receiver, Sender};
use light_curve_feature::{time_series::TimeSeries, FeatureEvaluator, FeatureExtractor};
use light_curve_interpol::Interpolator;
use num_cpus;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::iter::Iterator;
use std::thread;

#[derive(Clone)]
struct FluxDump {
    path: String,
    interpolator: Interpolator<f32, f32>,
    passbands: Vec<Passband>,
}

impl Dump for FluxDump {
    fn eval(&self, source: &Source) -> Vec<u8> {
        let mut result = vec![];
        for &passband in self.passbands.iter() {
            let lc = source.lc(passband);
            let flux: Vec<_> = lc.mag.iter().map(|&x| 10_f32.powf(-0.4 * x)).collect();
            self.interpolator
                .interpolate(&lc.t[..], &flux[..])
                .iter()
                .for_each(|x| {
                    let bytes = x.to_bits().to_ne_bytes();
                    result.extend_from_slice(&bytes);
                });
        }
        result
    }

    fn get_names(&self) -> Vec<&str> {
        vec![]
    }

    fn get_value_path(&self) -> &str {
        self.path.as_str()
    }

    fn get_name_path(&self) -> Option<&str> {
        None
    }
}

#[derive(Clone)]
struct FeatureDump {
    value_path: String,
    name_path: String,
    magn_feature_extractor: FeatureExtractor<f32>,
    flux_feature_extractor: FeatureExtractor<f32>,
    passbands: Vec<Passband>,
    names: Vec<String>,
}

impl FeatureDump {
    fn new(
        value_path: String,
        name_path: String,
        magn_feature_extractor: FeatureExtractor<f32>,
        flux_feature_extractor: FeatureExtractor<f32>,
        passbands: Vec<Passband>,
    ) -> Self {
        let magn_feature_extractor_names = magn_feature_extractor.get_names();
        let flux_feature_extractor_names = flux_feature_extractor.get_names();
        let extr_names_types = [
            (&magn_feature_extractor_names, "magn"),
            (&flux_feature_extractor_names, "flux"),
        ];
        let names = passbands
            .iter()
            .flat_map(|passband| {
                extr_names_types.iter().flat_map(
                    move |(feature_extractor_names, brightness_type)| {
                        feature_extractor_names
                            .iter()
                            .map(move |name| format!("{}_{}_{}", name, brightness_type, passband))
                    },
                )
            })
            .collect();
        Self {
            value_path,
            name_path,
            magn_feature_extractor,
            flux_feature_extractor,
            passbands,
            names,
        }
    }
}

impl Dump for FeatureDump {
    fn eval(&self, source: &Source) -> Vec<u8> {
        let mut result = vec![];
        for &passband in self.passbands.iter() {
            let lc = source.lc(passband);
            let flux: Vec<_> = lc.mag.iter().map(|&x| 10_f32.powf(-0.4 * x)).collect();
            let ts_magn = TimeSeries::new(&lc.t, &lc.mag, Some(&lc.w));
            let ts_flux = TimeSeries::new(&lc.t, &flux, Some(&lc.w));
            for (feature_extractor, ts) in &mut [
                (&self.magn_feature_extractor, ts_magn),
                (&self.flux_feature_extractor, ts_flux),
            ] {
                feature_extractor
                    .eval(ts)
                    .expect("Some feature cannot be extracted")
                    .iter()
                    .for_each(|x| {
                        let bytes = x.to_bits().to_ne_bytes();
                        result.extend_from_slice(&bytes);
                    });
            }
        }
        result
    }

    fn get_names(&self) -> Vec<&str> {
        self.names.iter().map(|s| s.as_str()).collect()
    }

    fn get_value_path(&self) -> &str {
        self.value_path.as_str()
    }

    fn get_name_path(&self) -> Option<&str> {
        Some(self.name_path.as_str())
    }
}

#[derive(Clone)]
struct SIDDump {
    path: String,
}

impl Dump for SIDDump {
    fn eval(&self, source: &Source) -> Vec<u8> {
        source.sid.to_ne_bytes().to_vec()
    }

    fn get_names(&self) -> Vec<&str> {
        vec![]
    }

    fn get_value_path(&self) -> &str {
        self.path.as_str()
    }

    fn get_name_path(&self) -> Option<&str> {
        None
    }
}

pub struct Dumper {
    passbands: Vec<Passband>,
    dumps: Vec<Box<dyn Dump + 'static>>,
    write_caches: Vec<Box<dyn Cache>>,
}

impl Dumper {
    pub fn new(passbands: &[Passband]) -> Self {
        Self {
            passbands: passbands.to_vec(),
            dumps: vec![],
            write_caches: vec![],
        }
    }

    pub fn set_sid_writer(&mut self, sid_path: String) -> &mut Self {
        self.dumps.push(Box::new(SIDDump { path: sid_path }));
        self
    }

    pub fn set_interpolator(
        &mut self,
        flux_path: String,
        interpolator: Interpolator<f32, f32>,
    ) -> &mut Self {
        self.dumps.push(Box::new(FluxDump {
            path: flux_path,
            interpolator,
            passbands: self.passbands.clone(),
        }));
        self
    }

    pub fn set_feature_extractor(
        &mut self,
        value_path: String,
        name_path: String,
        magn_feature_extractor: FeatureExtractor<f32>,
        flux_feature_extractor: FeatureExtractor<f32>,
    ) -> &mut Self {
        self.dumps.push(Box::new(FeatureDump::new(
            value_path,
            name_path,
            magn_feature_extractor,
            flux_feature_extractor,
            self.passbands.clone(),
        )));
        self
    }

    pub fn set_write_cache(&mut self, cache: Box<dyn Cache>) -> &mut Self {
        self.write_caches.push(cache);
        self
    }

    fn writer_from_path(path: &str) -> BufWriter<File> {
        let file = File::create(path).unwrap();
        BufWriter::new(file)
    }

    fn dump_eval_worker(
        dumps: Vec<Box<dyn Dump>>,
        receiver: Receiver<Source>,
        sender: Sender<Vec<Vec<u8>>>,
    ) {
        while let Ok(source) = receiver.recv() {
            let results = dumps.iter().map(|dump| dump.eval(&source)).collect();
            sender
                .send(results)
                .expect("Cannot send evaluation result to writer");
        }
    }

    fn dump_writer_worker(dumps: Vec<Box<dyn Dump>>, receiver: Receiver<Vec<Vec<u8>>>) {
        let mut writers: Vec<_> = dumps
            .iter()
            .map(|dump| Self::writer_from_path(dump.get_value_path()))
            .collect();
        while let Ok(data) = receiver.recv() {
            for (x, writer) in data.iter().zip(writers.iter_mut()) {
                writer.write(&x[..]).expect("Cannot write to file");
            }
        }
    }

    fn cache_writer_worker(receiver: Receiver<Source>, cache: Box<dyn Cache>) {
        let mut writer = cache.writer();

        while let Ok(source) = receiver.recv() {
            writer.write(&source);
        }
    }

    pub fn dump_query_iter(&self, source_iter: impl Iterator<Item = Source>) {
        const CHANNEL_CAP: usize = 1 << 10;

        let (dump_eval_sender, dump_eval_receiver) = bounded_channel(CHANNEL_CAP);
        let (dump_writer_sender, dump_writer_receiver) = bounded_channel(CHANNEL_CAP);
        let (cache_writer_senders, cache_writer_receivers): (Vec<_>, Vec<_>) = self
            .write_caches
            .iter()
            .map(|_| bounded_channel(CHANNEL_CAP))
            .unzip();

        let dump_eval_thread_pool: Vec<_> = (0..num_cpus::get())
            .map(|_| {
                let dumps = self.dumps.clone();
                let receiver = dump_eval_receiver.clone();
                let sender = dump_writer_sender.clone();
                thread::spawn(move || Self::dump_eval_worker(dumps, receiver, sender))
            })
            .collect();
        // Remove channel parts that are cloned and moved to workers
        drop(dump_eval_receiver);
        drop(dump_writer_sender);

        let dumps = self.dumps.clone();
        let dump_writer_thread =
            thread::spawn(move || Self::dump_writer_worker(dumps, dump_writer_receiver));

        let cache_write_thread_pool: Vec<_> = self
            .write_caches
            .iter()
            .map(|cache| cache.clone())
            .zip(cache_writer_receivers.into_iter())
            .map(|(cache, receiver)| {
                thread::spawn(move || Self::cache_writer_worker(receiver, cache))
            })
            .collect();

        for source in source_iter {
            for sender in cache_writer_senders.iter() {
                sender
                    .send(source.clone())
                    .expect("Cannot send task to cache worker");
            }
            // Send source to eval worker pool
            dump_eval_sender
                .send(source)
                .expect("Cannot send task to eval worker");
        }

        // Remove senders or writer_thread will never join
        drop(dump_eval_sender);
        drop(cache_writer_senders);
        for thread in dump_eval_thread_pool {
            thread.join().expect("Dumper eval worker panicked");
        }
        dump_writer_thread
            .join()
            .expect("Dumper writer worker panicked");
        for thread in cache_write_thread_pool {
            thread.join().expect("Dumper cache writer worker panicked");
        }
    }

    pub fn write_names(&self) -> usize {
        self.dumps
            .iter()
            .filter_map(|dump| dump.get_name_path().and_then(|path| Some((dump, path))))
            .map(|(dump, path)| {
                let mut writer = Self::writer_from_path(path);
                dump.get_names()
                    .iter()
                    .map(|name| {
                        writer.write(name.as_bytes()).unwrap() + writer.write(b"\n").unwrap()
                    })
                    .sum::<usize>()
            })
            .sum()
    }
}
