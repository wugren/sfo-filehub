import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const dist = join(process.cwd(), "dist");
const index = join(dist, "index.html");
const html = readFileSync(index, "utf8");
if (!html.includes('<div id="root">')) {
  throw new Error(`dist/index.html 缺少应用挂载点: ${index}`);
}
const assetsDir = join(dist, "assets");
const assets = (readdirSync(assetsDir) ?? []).filter((name) =>
  name.endsWith(".js") || name.endsWith(".css"),
);
if (assets.length === 0) {
  throw new Error(`构建产物缺少 js/css 资源: ${assetsDir}`);
}
for (const name of assets) {
  const file = join(assetsDir, name);
  if (statSync(file).size === 0) {
    throw new Error(`构建资源为空文件: ${file}`);
  }
}
console.log(`dv verify: dist ok (${assets.join(", ")})`);
