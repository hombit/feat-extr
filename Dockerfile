FROM rust:1.70-bookworm AS build

ENV RUSTFLAGS "-C target-cpu=native"

# Install FFTW3, HDF5 and Ceres
RUN apt-get update \
    && apt-get install -y libfftw3-dev libhdf5-dev libceres-dev \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir /app
WORKDIR /app

COPY Cargo.toml Cargo.lock /app/
RUN mkdir -p /app/src/bin \
    && echo "fn main(){}" > /app/src/bin/main.rs \
    && touch /app/src/lib.rs \
    && cargo build --release --no-default-features --features hdf,fftw-system,ceres-system \
    && cargo clean --release -p feat_extr \
    && rm -r /app/src

COPY ./src/ /app/src/
RUN cargo build --release --no-default-features --features hdf,fftw-system,ceres-system

###################
FROM debian:bookworm-slim

# No output from Ceres
ENV GLOG_minloglevel=4

# Install FFTW3, HDF5 and Ceres
RUN apt-get update \
    && apt-get install -y libfftw3-dev libhdf5-dev libceres-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/feat_extr /app

VOLUME /data

CMD ["/app", "--dir=/data"]
