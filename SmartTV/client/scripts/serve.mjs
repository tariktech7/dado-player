import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import path from "node:path";
const root = path.resolve("dist"),
  port = Number(process.env.PORT || 18497);
createServer(async (req, res) => {
  try {
    const url = new URL(req.url, "http://localhost"),
      file = path.resolve(
        root,
        "." +
          decodeURIComponent(
            url.pathname === "/" ? "/index.html" : url.pathname,
          ),
      );
    if (!file.startsWith(root + path.sep)) throw 0;
    const data = await readFile(file);
    res.writeHead(200, {
      "Content-Type":
        {
          ".html": "text/html",
          ".mp4": "video/mp4",
          ".js": "application/javascript",
          ".css": "text/css",
          ".svg": "image/svg+xml",
          ".png": "image/png",
          ".ttf": "font/ttf",
          ".json": "application/json",
        }[path.extname(file)] || "application/octet-stream",
    });
    res.end(data);
  } catch {
    res.writeHead(404);
    res.end();
  }
}).listen(port, "127.0.0.1", () =>
  console.log(`Preview: http://127.0.0.1:${port}`),
);
