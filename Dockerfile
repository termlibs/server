FROM rust:alpine3.20 AS build

WORKDIR /root/build
COPY . /root/build

SHELL ["/bin/ash", "-eo", "pipefail", "-c"]
RUN \
  apk update && \
  apk add musl-dev && \
  cargo build --release

FROM scratch AS runtime

COPY --from=build /root/build/target/release/termlib-server /

ENTRYPOINT ["/termlib-server"]
