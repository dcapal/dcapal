import { execFileSync } from "node:child_process";
import { readFile, readdir, rm, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

import coverageLib from "istanbul-lib-coverage";
import reportLib from "istanbul-lib-report";
import reports from "istanbul-reports";

const { createCoverageMap } = coverageLib;
const { createContext } = reportLib;

const frontendRoot = path.resolve(process.cwd());
const repoRoot = path.resolve(frontendRoot, "..");
const browserCoverageDir = path.join(frontendRoot, "coverage/playwright");
const apiCoverageFile = path.join(
  repoRoot,
  "packages/api-client/coverage/coverage-final.json"
);
const reportDir = path.join(repoRoot, "coverage/frontend");

const generatedPathParts = [
  "/src/gen/",
  "/src/gen-mocks/",
  "/src/model/",
  "/src/mocks/",
];

const toPosix = (value) => String(value).replaceAll("\\", "/");

const normalizeCoveragePath = (filePath) => {
  let value = decodeURIComponent(String(filePath));
  if (value.startsWith("file://")) value = new URL(value).pathname;
  value = toPosix(value);

  const webpackSource = value.lastIndexOf("/src/");
  if (webpackSource >= 0 && value.startsWith("webpack://")) {
    value = value.slice(webpackSource + 1);
  }

  if (value.startsWith("webpack://")) value = value.replace(/^webpack:\/?/, "");
  value = value.replace(/^\.\//, "");

  if (value.startsWith("dcapal-frontend/src/")) {
    return path.resolve(repoRoot, value);
  }

  if (value.startsWith("packages/api-client/src/")) {
    return path.resolve(repoRoot, value);
  }

  if (path.isAbsolute(value)) return path.normalize(value);
  return path.resolve(repoRoot, value);
};

const isInScope = (filePath) => {
  const relativePath = toPosix(path.relative(repoRoot, filePath));
  if (
    !(
      relativePath.startsWith("dcapal-frontend/src/") ||
      relativePath.startsWith("packages/api-client/src/")
    )
  ) {
    return false;
  }

  if (
    generatedPathParts.some((part) => `/${relativePath}`.includes(part)) ||
    /\.test\.[jt]sx?$/.test(relativePath) ||
    relativePath.endsWith(".d.ts")
  ) {
    return false;
  }

  return true;
};

const readJson = async (filePath) =>
  JSON.parse(await readFile(filePath, "utf8"));

const mergeCoverageFile = (coverageMap, filePath, fileCoverage) => {
  const normalizedPath = normalizeCoveragePath(filePath);
  if (!isInScope(normalizedPath)) return false;

  coverageMap.addFileCoverage({
    ...fileCoverage,
    path: normalizedPath,
  });
  return true;
};

const mergeCoverageObject = (coverageMap, coverageObject) => {
  let filesMerged = 0;
  for (const [filePath, fileCoverage] of Object.entries(coverageObject)) {
    if (mergeCoverageFile(coverageMap, filePath, fileCoverage))
      filesMerged += 1;
  }
  return filesMerged;
};

const getBaseRef = () => {
  if (process.env.COVERAGE_BASE_REF) return process.env.COVERAGE_BASE_REF;

  try {
    execFileSync("git", ["rev-parse", "--verify", "origin/master"], {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "ignore"],
    });
    return "origin/master";
  } catch {
    return "HEAD^";
  }
};

const parseChangedLines = (diff) => {
  const changedLines = new Map();
  let currentFile = null;

  for (const line of diff.split("\n")) {
    if (line.startsWith("+++ b/")) {
      currentFile = line.slice("+++ b/".length);
      continue;
    }

    if (!currentFile || !line.startsWith("@@")) continue;

    const match = line.match(/@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@/);
    if (!match) continue;

    const start = Number(match[1]);
    const count = match[2] === undefined ? 1 : Number(match[2]);
    const fileLines = changedLines.get(currentFile) || new Set();
    for (let lineNumber = start; lineNumber < start + count; lineNumber += 1) {
      fileLines.add(lineNumber);
    }
    changedLines.set(currentFile, fileLines);
  }

  return changedLines;
};

const getChangedLines = () => {
  const baseRef = getBaseRef();
  let diff;
  try {
    diff = execFileSync("git", ["diff", "--unified=0", baseRef], {
      cwd: repoRoot,
      encoding: "utf8",
    });
  } catch (error) {
    throw new Error(`Could not read the diff against ${baseRef}: ${error}`);
  }

  return { baseRef, changedLines: parseChangedLines(diff) };
};

const isNonExecutableSourceLine = (line) => {
  const trimmed = line.trim();
  return (
    trimmed === "" ||
    trimmed.startsWith("//") ||
    trimmed.startsWith("*") ||
    trimmed.startsWith("/*") ||
    trimmed === "*/" ||
    trimmed === "{" ||
    trimmed === "}" ||
    trimmed === ");"
  );
};

const getCoveredFunctionLines = (coverage, sourceLines) => {
  if (!coverage) return new Set();

  const coveredLines = new Set();
  const { fnMap = {}, f = {} } = coverage.data;

  for (const [functionId, functionInfo] of Object.entries(fnMap)) {
    if (!(f[functionId] > 0)) continue;

    const declarationLine = functionInfo.decl?.start?.line;
    const startLine = functionInfo.loc?.start?.line;
    const endLine = functionInfo.loc?.end?.line;
    const declaration = sourceLines[declarationLine - 1] || "";

    // Babel's transpiled async and arrow functions can report the function
    // hit while leaving the original source statements at zero. Only use
    // this fallback for source lines that are visibly function declarations.
    if (!declaration.includes("=>") && !declaration.includes("function")) {
      continue;
    }

    if (!startLine || !endLine) continue;
    for (let line = startLine; line <= endLine; line += 1) {
      coveredLines.add(line);
    }
  }

  return coveredLines;
};

const getChangedLineReport = async (coverageMap) => {
  const { baseRef, changedLines } = getChangedLines();
  const rows = [];

  for (const [relativePath, lineNumbers] of changedLines) {
    const absolutePath = path.resolve(repoRoot, relativePath);
    if (!isInScope(absolutePath)) continue;

    const sourceLines = (await readFile(absolutePath, "utf8")).split("\n");
    const coverage = coverageMap.data[absolutePath];
    const lineCoverage = coverage?.getLineCoverage() || {};
    const executableLines = Object.keys(lineCoverage).map(Number);
    const coveredFunctionLines = getCoveredFunctionLines(coverage, sourceLines);

    for (const lineNumber of [...lineNumbers].sort((a, b) => a - b)) {
      const sourceLine = sourceLines[lineNumber - 1] || "";
      if (isNonExecutableSourceLine(sourceLine)) continue;

      const isExecutable = coverage
        ? executableLines.includes(lineNumber)
        : true;
      if (!isExecutable) continue;

      rows.push({
        file: relativePath,
        line: lineNumber,
        covered:
          (lineCoverage[lineNumber] || 0) > 0 ||
          coveredFunctionLines.has(lineNumber),
      });
    }
  }

  rows.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);
  const covered = rows.filter((row) => row.covered).length;
  const header = [
    "# Changed-line coverage",
    "",
    `Base comparison: \`${baseRef}\``,
    "",
    "| File | Line | Status |",
    "| --- | ---: | --- |",
  ];
  const body = rows.map(
    (row) =>
      `| \`${row.file}\` | ${row.line} | ${row.covered ? "Covered" : "Uncovered"} |`
  );
  const summary = `\nChanged executable lines: **${covered}/${rows.length} covered**.`;

  return {
    markdown: [...header, ...body, summary].join("\n") + "\n",
    covered,
    total: rows.length,
  };
};

const formatPercentage = (covered, total) =>
  total === 0 ? "100.00%" : `${((covered / total) * 100).toFixed(2)}%`;

const writeTextSummary = async (coverageMap) => {
  const summary = coverageMap.getCoverageSummary().toJSON();
  const rows = [
    ["Statements", summary.statements.covered, summary.statements.total],
    ["Branches", summary.branches.covered, summary.branches.total],
    ["Functions", summary.functions.covered, summary.functions.total],
    ["Lines", summary.lines.covered, summary.lines.total],
  ];
  const text = [
    "Frontend and API-client coverage",
    "",
    "Metric       Covered  Total  Percentage",
    ...rows.map(
      ([label, covered, total]) =>
        `${label.padEnd(11)} ${String(covered).padStart(7)} ${String(total).padStart(6)}  ${formatPercentage(covered, total)}`
    ),
    "",
    "No global percentage threshold is enforced.",
  ].join("\n");

  await writeFile(path.join(reportDir, "text.txt"), `${text}\n`, "utf8");
  return text;
};

const main = async () => {
  const coverageMap = createCoverageMap({});

  let browserFragments = 0;
  try {
    const fragmentNames = await readdir(browserCoverageDir);
    for (const fragmentName of fragmentNames.filter((name) =>
      name.endsWith(".json")
    )) {
      browserFragments += mergeCoverageObject(
        coverageMap,
        await readJson(path.join(browserCoverageDir, fragmentName))
      );
    }
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }

  let apiFiles = 0;
  try {
    apiFiles = mergeCoverageObject(
      coverageMap,
      await readJson(apiCoverageFile)
    );
  } catch (error) {
    if (error.code === "ENOENT") {
      throw new Error(`API-client coverage is missing: ${apiCoverageFile}`);
    }
    throw error;
  }

  if (browserFragments === 0 || apiFiles === 0) {
    throw new Error(
      `Expected browser and API coverage (browser files: ${browserFragments}, API files: ${apiFiles})`
    );
  }

  await rm(reportDir, { recursive: true, force: true });
  await mkdir(reportDir, { recursive: true });

  const context = createContext({
    dir: reportDir,
    coverageMap,
    defaultSummarizer: "nested",
    projectRoot: repoRoot,
  });
  for (const reporterName of ["html", "lcovonly", "json", "json-summary"]) {
    reports.create(reporterName, { projectRoot: repoRoot }).execute(context);
  }

  const textSummary = await writeTextSummary(coverageMap);
  const changedLineReport = await getChangedLineReport(coverageMap);
  await writeFile(
    path.join(reportDir, "changed-lines.md"),
    changedLineReport.markdown,
    "utf8"
  );

  process.stdout.write(`${textSummary}\n\n${changedLineReport.markdown}`);
  if (process.env.GITHUB_STEP_SUMMARY) {
    await writeFile(
      process.env.GITHUB_STEP_SUMMARY,
      `\n${changedLineReport.markdown}`,
      { encoding: "utf8", flag: "a" }
    );
  }
};

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
