# Build image
# Necessary dependencies to build blot
FROM rust:alpine AS build

LABEL version="1.0.0" maintainer="Arei2<contact@arei2.fr>"

RUN apk update && apk upgrade
RUN apk add --no-cache \
    musl-dev alpine-sdk build-base \
    postgresql-dev perl perl-dev openssl-dev libssl3 libcrypto3 openssl-libs-static \
    pkgconfig

WORKDIR "/blot"

COPY . .

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target x86_64-unknown-linux-musl --release

# Release image
# Necessary dependencies to run blot
FROM alpine:latest

RUN apk add --no-cache --update tzdata && apk add docker-cli && apk add docker-cli-compose
ENV TZ=Europe/Paris
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

WORKDIR "/blot"

COPY --from=build /blot/target/x86_64-unknown-linux-musl/release/blot ./blot

CMD ["./blot"]