FROM rust:1.82

RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk --version 0.21.5

# TODO: this is pretty inefficient at this point,
# since build files aren't cached between runs
# and rebuilding all the dependencies is sloooooowww....
WORKDIR /app
COPY . .

EXPOSE 8080

WORKDIR intersect-glasses
CMD [ "trunk", "serve", "--release", "--address", "0.0.0.0" ]
