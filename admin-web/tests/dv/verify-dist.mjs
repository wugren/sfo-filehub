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

const requireSameOriginApi =
  process.env.GITHUB_ACTIONS === "true" || process.env.VITE_API_BASE_URL === "/";
if (requireSameOriginApi) {
  if (process.env.VITE_API_BASE_URL !== "/") {
    throw new Error(
      "Docker/CI 管理页面构建必须设置 VITE_API_BASE_URL=/，避免浏览器请求本机 API",
    );
  }

  const javascript = assets
    .filter((name) => name.endsWith(".js"))
    .map((name) => readFileSync(join(assetsDir, name), "utf8"))
    .join("\n");
  if (!javascript.includes("/account/login")) {
    throw new Error("构建产物缺少登录 API 路由: /account/login");
  }
  if (javascript.includes("http://127.0.0.1:8080")) {
    throw new Error(
      "Docker/CI 管理页面构建产物包含 loopback API 地址: http://127.0.0.1:8080",
    );
  }
}
console.log(`dv verify: dist ok (${assets.join(", ")})`);
