FROM rust as builder
WORKDIR build
COPY . .
RUN cargo build --release

FROM rust as runtime
MAINTAINER Kaspar Schleiser <kaspar@schleiser.de>
RUN apt-get update && apt-get install -y tini
ENTRYPOINT [ "/usr/bin/tini", "--" ]
CMD [ "/usr/bin/drone-riot-conv" ]
COPY --from=builder /build/target/release/drone-riot-conv /usr/bin
