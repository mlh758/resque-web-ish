FROM rust:1.35 as rust-build

WORKDIR /home/builder
RUN USER=root cargo new resque-web
WORKDIR /home/builder/resque-web
# Copy and build dependencies separately from app to improve subsequent build times
COPY plugin_manager ./plugin_manager
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
COPY src ./src
RUN touch -m src/main.rs
RUN cargo build --release


FROM node:10.16-stretch as node-build
ARG RELATIVE_URL=""
WORKDIR /home/builder/web-app
RUN npm install -g yarn
COPY web-app .
RUN yarn install
ENV PUBLIC_URL ${RELATIVE_URL}
RUN yarn build

FROM debian:stretch-slim
ARG RELATIVE_URL=""
WORKDIR /home/deploy/app
ENV SUB_URI ${RELATIVE_URL}
COPY --from=rust-build /home/builder/resque-web/target/release/resque-web /home/deploy/app/service
COPY --from=node-build /home/builder/web-app/build /home/deploy/app/public
EXPOSE 8080
ENTRYPOINT ["/home/deploy/app/service"]