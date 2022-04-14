#!/bin/bash

HOST=$1
DIR=$2
MINNOBS=$3
PASSBAND_STR=$4

if [[ "$PASSBAND_STR" == 'g' ]]; then
  PASSBAND_NUM=1
fi
if [[ "$PASSBAND_STR" == 'r' ]]; then
  PASSBAND_NUM=2
fi
if [[ "$PASSBAND_STR" == 'i' ]]; then
  PASSBAND_NUM=3
fi

NAME="short_${PASSBAND_STR}_${MINNOBS}"
SUFFIX="_${NAME}"

QUERY="
SELECT oid, mjd, filter, mag, magerr
  FROM ztf.dr8_obs
    WHERE mjd <= 58972.0 AND oid IN (SELECT oid
      FROM ztf.dr8_meta
      WHERE filter = ${PASSBAND_NUM} AND ngoodobs >= ${MINNOBS}
      )
      ORDER BY h3index10, oid, mjd
"

RUSTFLAGS="-Ctarget-cpu=native" cargo run --release --no-default-features --features fftw-mkl -- \
    clickhouse \
    "$QUERY" \
    --passbands=${PASSBAND_STR} \
    --dir=${DIR} \
    --suffix=${SUFFIX} \
    --connect="tcp://default@${HOST}:9000/ztf" \
    --sorted \
    --features
