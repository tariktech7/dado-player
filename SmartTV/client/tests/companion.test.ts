import test from "node:test";
import assert from "node:assert/strict";
// @ts-expect-error Shared Node runtime script is exercised directly.
import { allowedUrl, createCompanion } from "../scripts/companion.mjs";
test("companion rejects unexpected targets, clients, tokens and redirected hosts", async () => {
  assert.throws(() =>
    allowedUrl("file:///etc/passwd", ["https://provider.invalid"]),
  );
  assert.throws(() =>
    allowedUrl("http://127.0.0.1/secrets", ["https://provider.invalid"]),
  );
  const server = createCompanion(
    {
      token: "a".repeat(32),
      providerOrigins: ["https://provider.invalid"],
      clientOrigins: ["null"],
    },
    async () =>
      new Response("", {
        status: 302,
        headers: { Location: "http://127.0.0.1/private" },
      }),
  );
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  try {
    assert.equal(
      (
        await fetch(base + "/fetch", {
          method: "POST",
          headers: { Origin: "https://evil.invalid" },
        })
      ).status,
      403,
    );
    assert.equal(
      (
        await fetch(base + "/fetch", {
          method: "POST",
          headers: { Origin: "null" },
        })
      ).status,
      401,
    );
    const r = await fetch(base + "/fetch", {
      method: "POST",
      headers: { Origin: "null", Authorization: "Bearer " + "a".repeat(32) },
      body: JSON.stringify({
        url: "https://provider.invalid/api?password=SECRET",
      }),
    });
    assert.equal(r.status, 502);
    assert.ok(!(await r.text()).includes("SECRET"));
  } finally {
    await new Promise<void>((resolve) => server.close(resolve));
  }
});
