#!/bin/bash

HOST=$1
DIR=$2
MINNOBS=$3
NSRC=$4

NAME="gr_${MINNOBS}_${NSRC}"
SUFFIX="_${NAME}"

RUSTFLAGS="-Ctarget-cpu=native" cargo run --release -- \
    clickhouse \
    "SELECT sid, mjd, filter, mag, magerr
      FROM ztf.dr4_source_obs_02
      WHERE sid IN (SELECT sid
        FROM ztf.dr4_source_meta_02
	WHERE nobs_g >= ${MINNOBS} AND nobs_r >= ${MINNOBS}
	ORDER BY cityHash64(sid)
	LIMIT ${NSRC}
      )
      ORDER BY sid, mjd

" \
    --dir=${DIR} \
    --suffix=${SUFFIX} \
    --connect="tcp://default@${HOST}:9000/ztf" \
    --sorted \
    --features # \
    # --cache=-
