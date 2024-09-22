#syntax=docker/dockerfile:1.5
FROM rust:alpine3.20 AS build

WORKDIR /root/build
COPY . /root/build

SHELL ["/bin/ash", "-eo", "pipefail", "-c"]
RUN <<INSTRUCT
apk update
apk add musl-dev
cargo build --release
INSTRUCT

FROM scratch AS runtime

COPY --from=build /root/build/target/release/termlib-server /

ENTRYPOINT ["/termlib-server"]
