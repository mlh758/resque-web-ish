FROM rust:1.35 as rust-build

WORKDIR /home/builder
RUN USER=root cargo new resque-web
WORKDIR /home/builder/resque-web
# Copy and build dependencies separately from app to improve subsequent build times
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
COPY src ./src
RUN touch -m src/main.rs
RUN cargo build --release

FROM node:10.16-stretch as node-build
WORKDIR /home/builder/web-app
RUN npm install -g yarn
COPY web-app .
RUN yarn install
RUN yarn build

FROM debian:stretch-slim
WORKDIR /home/deploy/app
COPY --from=rust-build /home/builder/resque-web/target/release/resque-web /home/deploy/app/service
COPY --from=node-build /home/builder/web-app/build /home/deploy/app/public
EXPOSE 8080
ENTRYPOINT ["/home/deploy/app/service"]