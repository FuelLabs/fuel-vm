FROM gcr.io/oss-fuzz-base/base-builder-rust:v1
RUN rustup install nightly && rustup default nightly
RUN rustup component add rust-src --toolchain nightly
ENV RUSTUP_TOOLCHAIN=nightly
COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
