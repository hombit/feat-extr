#!/bin/bash

FIELDID="$1"
if [[ ${FIELDID} == "" ]]; then
  echo "Specify fieldid as the first argument"
  exit 1
fi

NAME="$2"
if [[ ${NAME} == "" ]]; then
  NAME=${FIELDID}
fi

FILTER=2
MIN_NGOODOBS=50

SERVER=cygnus-g10.sai.msu.ru
REMOTE_LOCATION="${SERVER}:~/test-features"
LOCAL_LOCATION="./test-features"

SUFFIX="_${NAME}"

echo "fieldid = ${FIELDID}"
echo "name = ${NAME}"
echo "filter = ${FILTER}"
echo "ngoodobs >= ${MIN_NGOODOBS}"

time docker-compose run --rm clickhouse_cyg \
    clickhouse \
    "SELECT oid, mjd, mag, magerr
        FROM ztf.dr3
        WHERE fieldid = ${FIELDID}
          AND catflags = 0
          AND mjd <= 58483.0
          AND oid IN (SELECT oid
            FROM ztf.dr3_meta_short
            WHERE fieldid = ${FIELDID}
              AND filter = ${FILTER}
              AND ngoodobs >= ${MIN_NGOODOBS}
          )
        ORDER BY h3index10, oid, mjd
" \
    --suffix=${SUFFIX} \
    --connect="tcp://api@snad.sai.msu.ru:9000/ztf" \
    --sorted \
    --features \
    --cache=-

# scp "${REMOTE_LOCATION}/feature${SUFFIX}.{dat,name}" "${LOCAL_LOCATION}"/
