# Setup the development container image

ARG VARIANT="bullseye"
FROM mcr.microsoft.com/vscode/devcontainers/rust:1-${VARIANT}

# https://github.com/docker-library/buildpack-deps/blob/65d69325ad741cea6dee20781c1faaab2e003d87/debian/bullseye/curl/Dockerfile
RUN apt-get update && apt-get install -y --no-install-recommends \
		ca-certificates \
		curl \
		netbase \
		wget \
	&& rm -rf /var/lib/apt/lists/*

RUN set -ex; \
	if ! command -v gpg > /dev/null; then \
		apt-get update; \
		apt-get install -y --no-install-recommends \
			gnupg \
			dirmngr \
		; \
		rm -rf /var/lib/apt/lists/*; \
	fi

# https://github.com/docker-library/buildpack-deps/blob/65d69325ad741cea6dee20781c1faaab2e003d87/debian/bullseye/Dockerfile
# procps is very common in build systems, and is a reasonably small package
RUN apt-get update && apt-get install -y --no-install-recommends \
		git \
		procps \
	&& rm -rf /var/lib/apt/lists/*

# Begin using vscode instead of root
USER vscode

# Setup nightly, wasm32-wasi and general components
RUN rustup update && \
	rustup default nightly

RUN rustup component add rustfmt rust-src clippy-preview

RUN rustup update

RUN cargo install cargo-expand
RUN cargo install devserver

RUN rustup update