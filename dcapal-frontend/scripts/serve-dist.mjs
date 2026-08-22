import { createReadStream } from "node:fs";
import { promises as fs } from "node:fs";
import { createServer, request as createRequest } from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const frontendRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  ".."
);
const distRoot = path.join(frontendRoot, "dist");
const port = Number(process.env.PORT || 3000);

const contentTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".ico": "image/x-icon",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".webmanifest": "application/manifest+json; charset=utf-8",
  ".wasm": "application/wasm",
};

/** Forwards API calls to the local backend used by the full-stack smoke job. */
const proxyApiRequest = (request, response, requestUrl) => {
  const backendPath = `${requestUrl.pathname.replace(/^\/api/, "")}${requestUrl.search}`;
  const backendRequest = createRequest(
    {
      hostname: "127.0.0.1",
      port: 8080,
      path: backendPath,
      method: request.method,
      headers: { ...request.headers, host: "127.0.0.1:8080" },
    },
    (backendResponse) => {
      response.writeHead(
        backendResponse.statusCode || 502,
        backendResponse.headers
      );
      backendResponse.pipe(response);
    }
  );

  backendRequest.on("error", (error) => {
    console.error(`Backend proxy failed: ${error.message}`);
    if (!response.headersSent) response.writeHead(502);
    response.end();
  });
  request.pipe(backendRequest);
};

const isWithinDist = (candidate) => {
  const relativePath = path.relative(distRoot, candidate);
  return (
    relativePath &&
    !relativePath.startsWith("..") &&
    !path.isAbsolute(relativePath)
  );
};

/** Serves the production assets and falls back to the SPA entrypoint for routes. */
const server = createServer(async (request, response) => {
  const requestUrl = new URL(request.url || "/", `http://127.0.0.1:${port}`);
  const requestedPath = decodeURIComponent(requestUrl.pathname);
  if (requestedPath === "/api" || requestedPath.startsWith("/api/")) {
    proxyApiRequest(request, response, requestUrl);
    return;
  }

  if (request.method !== "GET" && request.method !== "HEAD") {
    response.writeHead(405, { Allow: "GET, HEAD" });
    response.end();
    return;
  }

  const candidate = path.resolve(distRoot, `.${requestedPath}`);
  const candidateIsSafe = isWithinDist(candidate);
  let filePath = candidateIsSafe
    ? candidate
    : path.join(distRoot, "index.html");

  try {
    if (!(await fs.stat(filePath)).isFile())
      filePath = path.join(distRoot, "index.html");
  } catch {
    filePath = path.join(distRoot, "index.html");
  }

  response.writeHead(200, {
    "Content-Type":
      contentTypes[path.extname(filePath)] || "application/octet-stream",
  });
  if (request.method === "HEAD") {
    response.end();
    return;
  }

  createReadStream(filePath).pipe(response);
});

server.listen(port, "127.0.0.1", () => {
  console.log(`Serving ${distRoot} on http://127.0.0.1:${port}`);
});
