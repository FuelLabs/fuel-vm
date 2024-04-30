FROM gcr.io/oss-fuzz-base/base-builder-rust:v1
#RUN apt-get update && apt-get install -y make autoconf automake libtool
COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
