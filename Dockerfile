FROM rust:alpine AS build

WORKDIR /root/build
COPY . /root/build

RUN cargo build --release

FROM scratch AS runtime

COPY --from=build /root/build/target/release/termlib-server /

ENTRYPOINT ["/termlib-server"]
