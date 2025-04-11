FROM gcr.io/oss-fuzz-base/base-builder-rust:e155522ceab10af0de09331bb56acf6e0534b95d290de5a8b97539260c3908a9
# RUN rustup toolchain install 1.85
# RUN rustup default 1.85
COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
