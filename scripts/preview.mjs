import http from "node:http";
import { readFile } from "node:fs/promises";
import { dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../src");
const types = { ".html": "text/html", ".css": "text/css", ".js": "text/javascript", ".json": "application/json", ".svg": "image/svg+xml" };
const port = Number(process.env.COUNTDOWN_PREVIEW_PORT || 4173);
const server = http.createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
    const path = resolve(root, `.${pathname === "/" ? "/index.html" : pathname}`);
    if (!path.startsWith(root + sep)) { response.writeHead(403).end(); return; }
    const bytes = await readFile(path);
    response.writeHead(200, { "Content-Type": `${types[extname(path)] || "application/octet-stream"}; charset=utf-8`, "Cache-Control": "no-store" });
    response.end(bytes);
  } catch { response.writeHead(404).end("Not found"); }
});
server.listen(port, "127.0.0.1", () => console.log(`倒计时预览：http://127.0.0.1:${port}（仅界面预览，不启用系统功能）`));
