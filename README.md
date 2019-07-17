# Resque Web

This is a re-implementation of the Resque Web application in Rust and React. The goal is to provide a faster
interface and a more flexible deployment story. This allows us to deploy to any url and completely independent
of the application running Resque.

For all the commands below you will either need to have exported a `REDIS_CONNECTION_STRING` environment variable
or pass that along to the `cargo run` commands when you start the app. This is the redis connection string the app
will use when connecting.

## Development

To start the application run `cargo run` in the root directory and `yarn start` in the web-app directory.
The webpack dev server will refresh automatically when you make changes to the front end. You will need to stop
the Rust server and run `cargo run` again to get changes to the back end.

## Production mode

1. Run `yarn build`  in the web-app directory to get the compiled assets for the front end application.
2. Run `cp -r ./build/* ../public/` to copy the static assets to the public folder of the server where the
   server expects to see the assets
3. Run `cargo run --release` in the root directory to compile the server in release mode and run it.

## Deployment

You can follow the above steps to build the release version of the assets and server. Place the binary
next to the public folder and run it.

You can also use the provided Dockerfile to get a minimal docker container with this already done for you.
