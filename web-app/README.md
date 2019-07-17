# Resque Web Front End

This project was bootstrapped with [Create React App](https://github.com/facebook/create-react-app).
Start the back end server with `cargo run`, the dev server for the front end automatically proxies request
to it.

## Available Scripts

In the project directory, you can run:

### `yarn start`

Runs the app in the development mode.
Open [http://localhost:3000](http://localhost:3000) to view it in the browser.

The page will reload if you make edits.
You will also see any lint errors in the console.

### `yarn test`

Launches the test runner in the interactive watch mode.
See the section about [running tests](https://facebook.github.io/create-react-app/docs/running-tests) for more information.

### `yarn build`

Builds the app for production to the `build` folder.
It correctly bundles React in production mode and optimizes the build for the best performance.

The build is minified and the filenames include the hashes. These are the files you will serve
in production from the Rust server in the parent directory.
