#!/bin/bash

HOST=$1
DIR=$2
MINNOBS=$3

NAME="extragal_short_gr_${MINNOBS}"
SUFFIX="_${NAME}"

RUSTFLAGS="-Ctarget-cpu=native" cargo run --release -- \
    clickhouse \
    "SELECT sid, mjd, filter, mag, magerr
      FROM ztf.dr4_source_obs_02
      WHERE mjd <= 58664.0 AND sid IN (SELECT sid
        FROM ztf.dr4_source_meta_short_02
        WHERE nobs_g >= ${MINNOBS} AND nobs_r >= ${MINNOBS} AND abs(asin(sin(0.4734773249532946) * sin(pi() / 180. * dec) + cos(0.4734773249532946) * cos(pi() / 180. * dec) * cos(pi() / 180. * ra - 3.366032882941064)) * 180. / pi()) > 15.
      )
      ORDER BY sid, mjd
" \
    --dir=${DIR} \
    --suffix=${SUFFIX} \
    --connect="tcp://default@${HOST}:9000/ztf" \
    --sorted \
    --features # \
    # --cache=-
