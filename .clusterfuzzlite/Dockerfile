FROM gcr.io/oss-fuzz-base/base-builder-rust:v1
RUN rustup install nightly-2024-09-24 && rustup default nightly-2024-09-24
RUN rustup component add rust-src --toolchain nightly-2024-09-24
ENV RUSTUP_TOOLCHAIN=nightly-2024-09-24
COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
