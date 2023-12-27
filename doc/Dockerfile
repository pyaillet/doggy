FROM tsl0922/ttyd:alpine as ttyd
FROM alpine:latest as fontcollector

# Install Fonts
RUN apk add --no-cache \
    --repository=http://dl-cdn.alpinelinux.org/alpine/edge/main \
    --repository=http://dl-cdn.alpinelinux.org/alpine/edge/community \
    --repository=http://dl-cdn.alpinelinux.org/alpine/edge/testing \
    font-adobe-source-code-pro font-source-code-pro-nerd \
    font-dejavu font-dejavu-sans-mono-nerd \
    font-fira-code font-fira-code-nerd \
    font-hack font-hack-nerd \
    font-ibm-plex-mono-nerd \
    font-inconsolata font-inconsolata-nerd \
    font-jetbrains-mono font-jetbrains-mono-nerd \
    font-liberation font-liberation-mono-nerd \
    font-noto \
    font-roboto-mono \
    font-ubuntu font-ubuntu-mono-nerd \
    font-noto-emoji

FROM rust:slim-bookworm as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock .
RUN \
    mkdir /app/src && \
    echo 'fn main() {}' > /app/src/main.rs && \
    cargo build --release && \
    rm -Rvf /app/src

COPY src /app/src
ENV DOGGY_GIT_INFO=preview
RUN \
    touch src/main.rs && \
    cargo build && \
    cargo build --release

FROM debian:bookworm-slim

RUN apt-get update

# Install Dependencies
RUN apt-get -y install ffmpeg chromium bash curl

# Add fonts
COPY --from=fontcollector /usr/share/fonts/ /usr/share/fonts

# Install latest ttyd
COPY --from=ttyd /usr/bin/ttyd /usr/bin/ttyd

COPY --from=builder /app/target/release/doggy /usr/bin/doggy
COPY --from=builder /app/target/debug/doggy /usr/bin/doggy-debug

# Expose port
EXPOSE 1976

# Create volume
VOLUME /vhs
WORKDIR /vhs

# Create user
RUN useradd -u 1976 -U -s /bin/false vhs
# Mimic alpine default color option
RUN echo 'alias ls="ls --color"' >> ~/.bashrc
# Install

# FIXME: Remove this when support for function keys is merged
COPY ./doc/vhs /usr/bin/
#RUN curl -L https://github.com/charmbracelet/vhs/releases/download/v0.7.1/vhs_0.7.1_amd64.deb -o /tmp/vhs_0.7.1_amd64.deb && \
#    dpkg -i /tmp/vhs_0.7.1_amd64.deb && \
#    rm /tmp/vhs_0.7.1_amd64.deb

ENV VHS_PORT "1976"
ENV VHS_HOST "0.0.0.0"
ENV VHS_GID "1976"
ENV VHS_UID "1976"
ENV VHS_KEY_PATH "/vhs/vhs"
ENV VHS_AUTHORIZED_KEYS_PATH ""
ENV VHS_NO_SANDBOX "true"

ENTRYPOINT ["/usr/bin/vhs"]

