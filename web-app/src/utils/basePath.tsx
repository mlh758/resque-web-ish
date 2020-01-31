declare let BASE_PATH: string;
const BASE = BASE_PATH.endsWith("/")
  ? BASE_PATH.slice(0, BASE_PATH.length - 1)
  : BASE_PATH;
export default BASE;
