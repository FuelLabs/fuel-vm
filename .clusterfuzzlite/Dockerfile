FROM gcr.io/oss-fuzz-base/base-builder-rust:v1
# Update Rust to version 1.93.0
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER clang
ENV RUSTUP_TOOLCHAIN nightly-2026-03-02

RUN rm -rf /rust/rustup/toolchains/nightly-2024-07-12-x86_64-unknown-linux-gnu && \
		rustup default nightly && rustup toolchain install nightly-2026-03-02 && \
		rustup component add rust-src && \
		cargo install cargo-fuzz && \
		# remoevs default toolchain
		rm -rf /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu && mkdir -p /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu && \
		# copy to where the Fuzz builder expects to find the toolchain
		cp -r /rust/rustup/toolchains/nightly-2026-03-02-x86_64-unknown-linux-gnu/* /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu/



COPY . $SRC/fuel-vm
WORKDIR fuel-vm
COPY .clusterfuzzlite/build.sh $SRC/
