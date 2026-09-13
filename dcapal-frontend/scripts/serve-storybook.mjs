import { createReadStream } from "node:fs";
import { access, stat } from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../storybook-static"
);
const port = Number(process.env.STORYBOOK_PORT ?? 6006);
const host = process.env.STORYBOOK_HOST ?? "127.0.0.1";

const contentTypes = {
  ".css": "text/css; charset=utf-8",
  ".gif": "image/gif",
  ".html": "text/html; charset=utf-8",
  ".ico": "image/x-icon",
  ".jpg": "image/jpeg",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
};

function resolveRequestPath(requestUrl) {
  const pathname = decodeURIComponent(
    new URL(requestUrl, "http://localhost").pathname
  );
  const requestedPath = path.normalize(path.join(root, pathname));

  if (
    requestedPath !== root &&
    !requestedPath.startsWith(`${root}${path.sep}`)
  ) {
    return null;
  }

  return requestedPath;
}

async function findFile(requestedPath) {
  const candidates = [requestedPath];

  if (requestedPath.endsWith(path.sep)) {
    candidates.push(path.join(requestedPath, "index.html"));
  }

  if (!path.extname(requestedPath)) {
    candidates.push(path.join(root, "index.html"));
  }

  for (const candidate of candidates) {
    try {
      const details = await stat(candidate);
      if (details.isFile()) return candidate;
    } catch {
      // Try the next candidate, including the Storybook shell fallback.
    }
  }

  return null;
}

const server = createServer(async (request, response) => {
  const requestedPath = request.url ? resolveRequestPath(request.url) : null;
  const filePath = requestedPath ? await findFile(requestedPath) : null;

  if (!filePath) {
    response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    response.end("Not found");
    return;
  }

  const extension = path.extname(filePath).toLowerCase();
  response.writeHead(200, {
    "cache-control": "no-cache",
    "content-type": contentTypes[extension] ?? "application/octet-stream",
  });
  createReadStream(filePath).pipe(response);
});

await access(root);
server.listen(port, host, () => {
  console.log(`Serving Storybook from ${root} at http://${host}:${port}`);
});
