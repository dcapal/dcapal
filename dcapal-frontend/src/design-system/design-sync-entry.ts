/**
 * Library entry used only by the design-sync build
 * (`scripts/build-design-system-bundle.mjs`).
 *
 * It re-exports the reusable design-system surface: the typed UI primitives
 * and the product patterns. The journey compositions, the Invest composition
 * demo, and the fixtures are intentionally absent, so they never reach the
 * imported Claude Design bundle.
 */
export * from "./design-sync-theme-scope";
export * from "./components/ui";
export * from "./components/patterns/allocation-after-list";
export * from "./components/patterns/allocation-bar";
export * from "./components/patterns/app-shell";
export * from "./components/patterns/drift-diagnostic";
export * from "./components/patterns/drift-overview";
export * from "./components/patterns/floating-action-menu";
export * from "./components/patterns/form-field";
export * from "./components/patterns/invest-format";
export * from "./components/patterns/invest-plan";
export * from "./components/patterns/invest-types";
export * from "./components/patterns/investment-path-actions";
export * from "./components/patterns/investment-plan-form";
export * from "./components/patterns/investment-proposal";
export * from "./components/patterns/investment-result";
export * from "./components/patterns/metric-card";
export * from "./components/patterns/page-header";
export * from "./components/patterns/performance-chart";
export * from "./components/patterns/portfolio-asset-card";
export * from "./components/patterns/portfolio-asset-collection";
export * from "./components/patterns/portfolio-asset-table";
export * from "./components/patterns/portfolio-footer";
export * from "./components/patterns/portfolio-icon-picker";
export * from "./components/patterns/portfolio-index";
export * from "./components/patterns/portfolio-setup";
export * from "./components/patterns/portfolio-workspace";
export * from "./components/patterns/portfolio-workspace-sidebar";
export * from "./components/patterns/proposed-transaction-collection";
export * from "./components/patterns/sticky-action-bar";
export * from "./components/patterns/transaction-fee-configuration";
