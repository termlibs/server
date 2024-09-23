FROM rust:alpine3.20 AS build

WORKDIR /root/build
COPY . /root/build

SHELL ["/bin/ash", "-eo", "pipefail", "-c"]
RUN \
  apk update && \
  apk add musl-dev && \
  cargo build --release

FROM alpine:3.20 AS runtime

USER 1000
WORKDIR /
COPY --chown=1000:1000 --from=build /root/build/target/release/termlib-server /usr/bin/termlib-server
COPY --chown=1000:1000 --from=build /root/build/termlibs /termlibs
ENTRYPOINT ["termlib-server"]
