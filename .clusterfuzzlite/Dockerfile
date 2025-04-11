FROM gcr.io/oss-fuzz-base/base-builder-rust:v1
# Update Rust to version 1.85.1
RUN rustup update stable --no-self-update && \
    rustup default 1.85.1
COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
