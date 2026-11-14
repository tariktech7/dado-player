import { createServer } from "node:http";
import { readFile, writeFile } from "node:fs/promises";
import { randomBytes, timingSafeEqual } from "node:crypto";
import { pathToFileURL } from "node:url";
export function allowedUrl(value, allowed) {
  const url = new URL(value);
  if (
    !["http:", "https:"].includes(url.protocol) ||
    !allowed.includes(url.origin)
  )
    throw new Error("Origin not allowed");
  return url;
}
export function createCompanion(config, fetcher = fetch) {
  if (
    !config.token ||
    config.token.length < 24 ||
    !Array.isArray(config.providerOrigins) ||
    !config.providerOrigins.length
  )
    throw new Error(
      "Set a random token and providerOrigins in the local configuration.",
    );
  return createServer(async (req, res) => {
    const origin = req.headers.origin || "";
    const cors = config.clientOrigins?.includes(origin)
      ? {
          "Access-Control-Allow-Origin": origin,
          Vary: "Origin",
          "Access-Control-Allow-Headers": "Authorization, Content-Type",
          "Access-Control-Allow-Methods": "POST, OPTIONS",
        }
      : {};
    const reply = (status, body) => {
      res.writeHead(status, {
        ...cors,
        "Content-Type": "application/json",
        "Cache-Control": "no-store",
      });
      res.end(JSON.stringify(body));
    };
    if (origin && !config.clientOrigins?.includes(origin)) {
      reply(403, { error: "Client origin not allowed" });
      return;
    }
    if (req.method === "OPTIONS") {
      res.writeHead(204, cors);
      res.end();
      return;
    }
    const actual = Buffer.from(req.headers.authorization || ""),
      expected = Buffer.from("Bearer " + config.token);
    if (
      actual.length !== expected.length ||
      !timingSafeEqual(actual, expected)
    ) {
      reply(401, { error: "Unauthorized" });
      return;
    }
    if (req.method !== "POST" || req.url !== "/fetch") {
      reply(404, { error: "Not found" });
      return;
    }
    try {
      let body = "";
      for await (const part of req) {
        body += part;
        if (body.length > 8192) throw new Error("Request too large");
      }
      const request = JSON.parse(body),
        url = allowedUrl(request.url, config.providerOrigins),
        controller = new AbortController(),
        timer = setTimeout(() => controller.abort(), 40000);
      try {
        let current = url,
          response;
        for (let i = 0; i < 6; i++) {
          response = await fetcher(current, {
            redirect: "manual",
            signal: controller.signal,
          });
          if (
            response.status >= 300 &&
            response.status < 400 &&
            response.headers.get("location")
          ) {
            current = allowedUrl(
              new URL(response.headers.get("location"), current).href,
              config.providerOrigins,
            );
            await response.body?.cancel();
            continue;
          }
          break;
        }
        if (!response?.ok || response.status >= 300) {
          reply(502, { error: "Provider request failed" });
          return;
        }
        const chunks = [];
        let bytes = 0;
        for await (const part of response.body) {
          bytes += part.length;
          if (bytes > 30_000_000) {
            controller.abort();
            throw new Error("Response too large");
          }
          chunks.push(part);
        }
        res.writeHead(200, {
          ...cors,
          "Content-Type":
            response.headers.get("content-type") || "application/json",
          "Cache-Control": "no-store",
        });
        res.end(Buffer.concat(chunks));
      } finally {
        clearTimeout(timer);
      }
    } catch {
      reply(502, {
        error: "Provider unreachable, origin not allowed, or invalid response",
      });
    }
  });
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  const configPath = process.argv[3] || "companion.local.json";
  if (process.argv[2] === "--init") {
    await writeFile(
      configPath,
      JSON.stringify(
        {
          host: "127.0.0.1",
          port: 18498,
          token: randomBytes(32).toString("hex"),
          providerOrigins: ["https://your-provider.example"],
          clientOrigins: ["null", "http://127.0.0.1:18497"],
        },
        null,
        2,
      ),
      { flag: "wx" },
    );
    console.log(
      `Created ${configPath}. Configure providerOrigins, TV origin and LAN bind address before use. Keep this file private.`,
    );
  } else {
    const config = JSON.parse(
      await readFile(process.argv[2] || "companion.local.json", "utf8"),
    );
    const server = createCompanion(config);
    server.listen(config.port || 18498, config.host || "127.0.0.1", () =>
      console.log(
        `Dado API/EPG companion listening on ${config.host || "127.0.0.1"}:${config.port || 18498}. No credentials or provider URLs are logged.`,
      ),
    );
  }
}
