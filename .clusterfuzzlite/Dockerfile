FROM gcr.io/oss-fuzz-base/base-builder-rust:v1
RUN rustup toolchain install 1.85
RUN rustup default 1.85
COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
