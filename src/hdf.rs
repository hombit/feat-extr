use crate::lc::{Observation, Source};
use crate::traits::{Cache, CacheWriter, ObservationsToSources};

use hdf5::Dataset;
use light_curve_feature::ndarray;

const DATASET_SIZE_STEP: hdf5::Ix = 1 << 16;
static DATASET_NAME: &'static str = "dataset";

#[derive(Clone)]
pub struct Hdf5Cache {
    pub path: String,
}

impl Hdf5Cache {
    fn dataset(&self, path: String, create: bool) -> hdf5::Result<hdf5::Dataset> {
        let dataset = if create {
            let file = hdf5::File::create(path)?;
            file.new_dataset::<Observation>()
                // .resizable(true)
                .shape([DATASET_SIZE_STEP])
                .create(DATASET_NAME)?
        } else {
            let file = hdf5::File::open(path)?;
            file.dataset(DATASET_NAME)?
        };
        Ok(dataset)
    }
}

impl Cache for Hdf5Cache {
    fn reader(&self) -> Box<dyn Iterator<Item = Source>> {
        let dataset = self.dataset(self.path.clone(), false).unwrap();
        let obs_reader = Hdf5ObservationReader::new(dataset);
        let source_reader = obs_reader.sources(true);
        Box::new(source_reader)
    }

    fn writer(&self) -> Box<dyn CacheWriter> {
        let dataset = self.dataset(self.path.clone(), true).unwrap();
        Box::new(Hdf5CacheWriter::new(dataset))
    }
}

struct Hdf5ObservationReader {
    dataset: Dataset,
    dataset_index: usize,
    dataset_size: usize,
    buffer: ndarray::Array1<Observation>,
    buffer_index: usize,
}

impl Hdf5ObservationReader {
    fn new(dataset: Dataset) -> Self {
        let size = dataset.size();
        Self {
            dataset,
            dataset_index: 0,
            dataset_size: size,
            buffer: ndarray::Array1::from(vec![]),
            buffer_index: 0,
        }
    }
}

impl Iterator for Hdf5ObservationReader {
    type Item = Observation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.dataset_index == self.dataset_size {
            return None;
        }

        if self.buffer_index == self.buffer.len() {
            let begin = self.dataset_index;
            let end = usize::min(self.dataset_index + DATASET_SIZE_STEP, self.dataset_size);
            let selection: hdf5::Selection = ndarray::s![begin..end].try_into().unwrap();

            self.buffer = self.dataset.as_reader().read_slice_1d(&selection).unwrap();
            self.buffer_index = 0;
        }

        let result = Some(self.buffer[self.buffer_index].clone());
        self.dataset_index += 1;
        self.buffer_index += 1;

        result
    }
}

impl ObservationsToSources for Hdf5ObservationReader {}

struct Hdf5CacheWriter {
    dataset: Dataset,
    index: usize,
    size: usize,
}

impl Hdf5CacheWriter {
    fn new(dataset: Dataset) -> Self {
        let size = dataset.size();
        Self {
            dataset,
            index: 0,
            size,
        }
    }
}

impl CacheWriter for Hdf5CacheWriter {
    fn write(&mut self, source: &Source) {
        let observations: Vec<_> = source.iter_observations().collect();

        let begin = self.index;
        self.index += observations.len();
        if self.index >= self.size {
            while self.size <= self.index {
                self.size += DATASET_SIZE_STEP;
            }
            self.dataset.resize(self.size).unwrap();
        }

        let selection: hdf5::Selection = ndarray::s![begin..begin + observations.len()].try_into().unwrap();
        self.dataset.write_slice(&observations, &selection).unwrap();
    }
}

impl Drop for Hdf5CacheWriter {
    fn drop(&mut self) {
        self.dataset.resize(self.index).unwrap();
    }
}
