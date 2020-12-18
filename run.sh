#!/bin/bash

NAME="test"

SUFFIX="_${NAME}"

cargo run --release -- \
    clickhouse \
    "SELECT sid, mjd, filter, mag, magerr
      FROM ztf.dr4_source_obs_1
      WHERE sid IN (SELECT sid
        FROM ztf.dr4_source_meta_1
        WHERE nobs_g >= 121 AND nobs_r >= 121
      )
      ORDER BY h3index10, sid, mjd

" \
    --suffix=${SUFFIX} \
    --connect="tcp://default@localhost:9000/ztf" \
    --sorted \
    --features \
    --cache=-
