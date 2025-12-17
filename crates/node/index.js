import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const binding = require("./maprender_node.node");

const { Renderer } = binding;

export { Renderer };
export default binding;
