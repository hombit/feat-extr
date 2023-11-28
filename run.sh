#!/bin/bash

HOST=$1
DIR=$2
MINNOBS=$3
PASSBANDS="${@:4}"

PASSBANDS_STR="$(printf '%s' ${PASSBANDS[@]})"

NAME="extragal_short_${PASSBANDS_STR}_${MINNOBS}"
SUFFIX="_${NAME}"

WHERE_AND_NOBS="$( printf " AND nobs_%s >= ${MINNOBS}" ${PASSBANDS[@]} )"
QUERY="
SELECT sid, mjd, filter, mag, magerr
  FROM ztf.dr4_source_obs_02
    WHERE mjd <= 58664.0 AND sid IN (SELECT sid
      FROM ztf.dr4_source_meta_short_02
      WHERE abs(asin(sin(0.4734773249532946) * sin(pi() / 180. * dec) + cos(0.4734773249532946) * cos(pi() / 180. * dec) * cos(pi() / 180. * ra - 3.366032882941064)) * 180. / pi()) > 15. ${WHERE_AND_NOBS}
      )
      ORDER BY sid, mjd
"

RUSTFLAGS="-Ctarget-cpu=native" cargo run --release --no-default-features --features fftw-mkl -- \
    clickhouse \
    "$QUERY" \
    --passbands=${PASSBANDS_STR} \
    --dir=${DIR} \
    --suffix=${SUFFIX} \
    --connect="tcp://api@${HOST}:9000/ztf" \
    --sorted \
    --features
