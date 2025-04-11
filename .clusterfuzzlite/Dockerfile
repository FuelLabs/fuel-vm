FROM gcr.io/oss-fuzz-base/base-builder-rust:9f439c9572c8dcc7f6f02d9a3f278363c1b19cd62d3fcfbedc45be866b3f0bdb
# RUN rustup toolchain install 1.85
# RUN rustup default 1.85
COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
