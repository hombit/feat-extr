#!/bin/bash

DIR=/data
MINNOBS=$1
PASSBAND_STR=$2
FEATURE_VERSION=$3
HOST=$4

if [[ "$PASSBAND_STR" == 'g' ]]; then
  PASSBAND_NUM=1
fi
if [[ "$PASSBAND_STR" == 'r' ]]; then
  PASSBAND_NUM=2
fi
if [[ "$PASSBAND_STR" == 'i' ]]; then
  PASSBAND_NUM=3
fi

NAME="${FEATURE_VERSION}_${PASSBAND_STR}_${MINNOBS}"
SUFFIX="_${NAME}"

QUERY="
WITH
    59184. AS mjd_min,
    59524. AS mjd_max
SELECT
    oid AS sid,
    mjd,
    filter,
    mag,
    magerr
FROM
(
    SELECT
        oid,
        filter,
        mjd,
        mag,
        magerr AS magerr
    FROM ztf.dr17_olc
    WHERE (filter = ${PASSBAND_NUM}) AND (arraySum(t -> ((t >= mjd_min) AND (t <= mjd_max)), mjd) >= ${MINNOBS}) AND (abs((asin((sin(0.4734773249532946) * sin((pi() / 180.) * dec)) + ((cos(0.4734773249532946) * cos((pi() / 180.) * dec)) * cos(((pi() / 180.) * ra) - 3.366032882941064))) * 180.) / pi()) > 15.)
)
ARRAY JOIN
    mjd,
    mag,
    magerr
WHERE (mjd >= mjd_min) AND (mjd <= mjd_max)
-- LIMIT 100
"

docker-compose run --rm clickhouse_cyg /app \
    clickhouse \
    "$QUERY" \
    --passbands=${PASSBAND_STR} \
    --dir=${DIR} \
    --suffix=${SUFFIX} \
    --connect="tcp://api@${HOST}:9000/ztf" \
    --sorted \
    --features \
    --feature-version=${FEATURE_VERSION}
