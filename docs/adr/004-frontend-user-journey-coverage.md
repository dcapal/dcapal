# Frontend coverage uses real user journeys at the browser boundary

Status: accepted

Frontend behavior will be covered primarily through Playwright user journeys running the real React application in Chromium, with MSW’s browser worker as the only backend boundary. The coverage report will combine those journeys with focused MSW-backed tests for handwritten API transport edge cases that have no honest user journey; every changed in-scope executable line must be covered, while function and branch totals remain diagnostics rather than merge gates. GitHub will publish the report and changed-line coverage without enforcing a global threshold. This keeps user-facing behavior realistic while making authentication, HTTP parsing, and retry branches testable without a live backend or production-only test routes.
