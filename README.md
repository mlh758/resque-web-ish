# Resque Web

This is a re-implementation of the Resque Web application in Rust and React. The goal is to provide a faster
interface and a more flexible deployment story. This allows us to deploy to any url and completely independent
of the application running Resque.

For all the commands below you will either need to have exported a `REDIS_CONNECTION_STRING` environment variable
or pass that along to the `cargo run` commands when you start the app. This is the redis connection string the app
will use when connecting. For details on the connection parameters, see the [Redis crate docs](https://docs.rs/redis/0.17.0/redis/#connection-parameters).

You can also provide configuration through the following variables which will be checked if the above variable is
not provided:

1. REDIS_HOSTNAME: hostname for Redis database
2. REDIS_DATABASE: defaults to 0
3. REDIS_PORT: defaults to 6379
4. REDIS_PASSWORD

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

### Deploying Under a Nested Path

You can build the docker image to use a nested URL by passing a build arg:

`docker build --build-arg RELATIVE_URL=/resque-web .`

This will cause the application to work as `https://example.com/resque-web` instead of expecting to be deployed
on the root path. If you are not using docker, you can use a build process identical to the one in the docker file
to build the assets with a PUBLIC_URL and set the environment variable for the back end.

## Plugins

The `plugin_manager` crate defined in this repository provides functionality to define and load plugins
for the web application. Plugins must adhere to the provided trait and will be loaded from the directory
incidated by the `RESQUE_PLUGIN_DIR` environment variable. Plugins are dynamic libraries compiled for the
current platform (so, dll, dylib...).

Plugins are called around some actions and give you the ability to extend the default behavior. For example
you may wish to delete additional keys in another Redis database when a queue is dropped.
