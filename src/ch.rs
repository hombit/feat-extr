use crate::lc::{Observation, Passband, MJD0};
use crate::traits::{ObservationsToSources, SourceDataBase};
use async_std::task;
use clickhouse_rs::errors::Error;
use clickhouse_rs::types::{Block, FromSql};
use clickhouse_rs::{ClientHandle, Pool};
use futures_util::stream::{BoxStream, StreamExt};

pub struct CHSourceDataBase {
    client: ClientHandle,
}

impl CHSourceDataBase {
    pub fn new(url: &str) -> Self {
        let pool = Pool::new(url);
        let client = task::block_on(pool.get_handle()).unwrap();
        Self { client }
    }
}

impl<'a> SourceDataBase<'a> for CHSourceDataBase {
    type Query = CHQuery<'a>;

    fn query(&'a mut self, query: &str) -> Self::Query {
        CHQuery::new(self, query)
    }
}

pub struct CHQuery<'a> {
    stream: BoxStream<'a, Result<Block, Error>>,
}

impl<'a> CHQuery<'a> {
    pub fn new(ch_db: &'a mut CHSourceDataBase, query: &str) -> Self {
        let stream = ch_db.client.query(query).stream_blocks();
        Self { stream }
    }
}

impl<'a> IntoIterator for CHQuery<'a> {
    type Item = Observation;
    type IntoIter = CHQueryIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(self)
    }
}

struct Row<'b> {
    block: &'b Block,
    idx: usize,
}

impl<'b> Row<'b> {
    fn get<T>(&self, col: &str) -> Result<T, Error>
    where
        T: FromSql<'b>,
    {
        self.block.get(self.idx, col)
    }
}

struct CurrentBlock {
    block: Block,
    size: usize,
    idx: usize,
}

impl CurrentBlock {
    fn new(block: Block) -> Self {
        let size = block.row_count();
        Self {
            block,
            size,
            idx: 0,
        }
    }
}

pub struct CHQueryIterator<'a> {
    query: CHQuery<'a>,
    block: Option<CurrentBlock>,
}

impl<'a> CHQueryIterator<'a> {
    fn new(query: CHQuery<'a>) -> Self {
        Self { query, block: None }
    }

    fn row_to_obs(row: Row) -> Observation {
        let sid: u64 = row.get("sid").unwrap();
        let filter: u8 = row.get("filter").unwrap();
        let mjd: f64 = row.get("mjd").unwrap();
        let t = (mjd - MJD0) as f32;
        let mag: f32 = row.get("mag").unwrap();
        let magerr: f32 = row.get("magerr").unwrap();
        let w = magerr.powi(-2);
        Observation {
            sid,
            passband: Passband::from_code(filter),
            t,
            mag,
            w,
        }
    }
}

impl<'a> Iterator for CHQueryIterator<'a> {
    type Item = Observation;

    fn next(&mut self) -> Option<Self::Item> {
        while self.block.is_none()
            || self.block.as_ref().unwrap().size == self.block.as_ref().unwrap().idx
        {
            match task::block_on(self.query.stream.next()) {
                Some(block) => self.block = Some(CurrentBlock::new(block.unwrap())),
                None => return None,
            }
        }

        match &mut self.block {
            Some(cur_block) => {
                cur_block.idx += 1;
                Some(Self::row_to_obs(Row {
                    block: &cur_block.block,
                    idx: cur_block.idx - 1,
                }))
            }
            None => panic!("We cannot be here"),
        }
    }
}

impl<'a> ObservationsToSources for CHQueryIterator<'a> {}
