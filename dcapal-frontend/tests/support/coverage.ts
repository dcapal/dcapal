import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import v8ToIstanbul from "v8-to-istanbul";
import type { Page, TestInfo } from "@playwright/test";

type BrowserCoverageEntries = Awaited<
  ReturnType<Page["coverage"]["stopJSCoverage"]>
>;

const normalizeSourcePath = (sourcePath: string): string => {
  // V8 reports webpack URLs while Istanbul expects repository-relative paths.
  let normalized = decodeURIComponent(String(sourcePath)).replace(/\\/g, "/");
  const srcMarker = normalized.lastIndexOf("/src/");

  if (srcMarker >= 0) {
    normalized = normalized.slice(srcMarker + 1);
  } else {
    normalized = normalized.replace(/^webpack:\/\/?/, "");
    normalized = normalized.replace(/^\.?\//, "");
  }

  if (normalized.startsWith("src/")) {
    return `dcapal-frontend/${normalized}`;
  }

  return normalized;
};

/** Writes one browser's V8 coverage as an Istanbul fragment for aggregation. */
export const writeBrowserCoverage = async (
  entries: BrowserCoverageEntries,
  testInfo: TestInfo
): Promise<void> => {
  const repoRoot = path.resolve(testInfo.config.rootDir, "../..");
  const coverageDir = path.resolve(
    testInfo.config.rootDir,
    "../coverage/playwright"
  );
  const fragment: Record<string, any> = {};

  for (const entry of entries) {
    if (!entry.url?.includes("127.0.0.1:3000") || !entry.source) continue;

    const converter = v8ToIstanbul(entry.url, 0, { source: entry.source });
    await converter.load();
    converter.applyCoverage(entry.functions);

    for (const [sourcePath, fileCoverage] of Object.entries(
      converter.toIstanbul()
    )) {
      const relativePath = normalizeSourcePath(sourcePath);
      if (!relativePath.startsWith("dcapal-frontend/src/")) continue;

      const absolutePath = path.resolve(repoRoot, relativePath);
      fragment[absolutePath] = {
        ...fileCoverage,
        path: absolutePath,
      };
    }
  }

  await mkdir(coverageDir, { recursive: true });
  const fileName = [
    testInfo.project.name,
    testInfo.workerIndex,
    testInfo.testId,
    testInfo.retry,
  ]
    .join("-")
    .replace(/[^a-zA-Z0-9._-]/g, "_");

  await writeFile(
    path.join(coverageDir, `${fileName}.json`),
    JSON.stringify(fragment),
    "utf8"
  );
};
