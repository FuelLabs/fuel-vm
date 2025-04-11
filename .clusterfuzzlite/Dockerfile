FROM gcr.io/oss-fuzz-base/base-builder-rust:v1
# Update Rust to version 1.85.1
RUN rustup install 1.85.1 && rustup default 1.85.1
RUN mkdir /rust/rustup/toolchains/1.85.1-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/
COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
