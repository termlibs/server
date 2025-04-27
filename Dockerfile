FROM rust:alpine3.20 AS build

RUN apk update && apk add musl-dev git

RUN  mkdir -p /root/termlibs && \
      git -C /root/termlibs clone --single-branch https://github.com/termlibs/install.sh.git && \
      git -C /root/termlibs clone --single-branch https://github.com/termlibs/json.sh.git && \
      git -C /root/termlibs clone --single-branch https://github.com/termlibs/logging.sh.git && \
      git -C /root/termlibs clone --single-branch https://github.com/termlibs/build.sh.git

WORKDIR /root/build
COPY . /root/build
RUN cargo build --release

FROM alpine:3.20 AS runtime

USER 1000
WORKDIR /web
COPY --chown=1000:1000 --from=build /root/build/target/release/termlib-server /usr/bin/termlib-server
COPY --chown=1000:1000 --from=build /root/termlibs /etc/termlibs
COPY --chown=1000:1000 ./templates /web/templates
ENV TERMLIBS_ROOT=/etc/termlibs
ENTRYPOINT ["termlib-server"]
