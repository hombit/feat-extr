#!/bin/bash

HOST=$1
DIR=$2
MINNOBS=$3

NAME="short_gr_${MINNOBS}"
SUFFIX="_${NAME}"

RUSTFLAGS="-Ctarget-cpu=native" cargo run --release -- \
    clickhouse \
    "SELECT sid, mjd, filter, mag, magerr
      FROM ztf.dr4_source_obs_02
      WHERE sid IN (SELECT sid
        FROM ztf.dr4_source_meta_short_02
        WHERE nobs_g >= ${MINNOBS} AND nobs_r >= ${MINNOBS}
      )
      ORDER BY sid, mjd

" \
    --dir=${DIR} \
    --suffix=${SUFFIX} \
    --connect="tcp://default@${HOST}:9000/ztf" \
    --sorted \
    --features # \
    # --cache=-
