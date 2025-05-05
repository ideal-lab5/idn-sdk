FROM paritytech/ci-linux:production as build

WORKDIR /code
COPY . .
# the latest release of ahash uses build_hasher_simple_hash_one 
# which was stabilized in 1.71 but the latest rustc version 
# from the paritytech/ci-linux:production is 1.68.0.
RUN rustup toolchain uninstall nightly
RUN rustup toolchain install nightly
RUN rustup target add wasm32-unknown-unknown --toolchain nightly-x86_64-unknown-linux-gnu
RUN rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
RUN cargo +nightly build --release

FROM ubuntu:22.04
WORKDIR /node

# Copy the node binary.
COPY --from=build /code/target/release/idn-node .

# Install root certs, see: https://github.com/paritytech/substrate/issues/9984
RUN apt update && \
    apt install -y ca-certificates && \
    update-ca-certificates && \
    apt remove ca-certificates -y && \
    rm -rf /var/lib/apt/lists/*

EXPOSE 9944
ENTRYPOINT ["./idn-node"]