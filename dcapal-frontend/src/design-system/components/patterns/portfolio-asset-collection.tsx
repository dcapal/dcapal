import * as React from "react";
import type { AssetClass, PortfolioAsset } from "../../fixtures/portfolio";
import { EmptyState, SegmentedControl } from "../ui";
import { PortfolioAssetCard } from "./portfolio-asset-card";
import { PortfolioAssetGroupCard } from "./portfolio-asset-group-card";
import { DriftBandProvider } from "./portfolio-asset-drift-badge";
import {
  buildPortfolioAssetSummary,
  DEFAULT_DRIFT_BAND_PP,
  type StrategicTargets,
  sortAssetEntries,
  sortAssetGroups,
} from "./portfolio-asset-groups";
import {
  PortfolioAssetTableOptions,
  portfolioAssetModeOptions,
} from "./portfolio-asset-table-options";
import { PortfolioAssetTable } from "./portfolio-asset-table";
import {
  usePortfolioAssetViewState,
  type PortfolioAssetMode,
} from "./use-portfolio-asset-view";

export interface PortfolioAssetCollectionProps {
  assets: readonly PortfolioAsset[];
  /** Selects a focused holdings summary or the full asset-management table. */
  presentation?: "detailed" | "overview";
  /** Controlled table mode; pair it with `onModeChange`. */
  mode?: PortfolioAssetMode;
  /** Initial table mode when the mode is not controlled. Defaults to `simple`. */
  defaultMode?: PortfolioAssetMode;
  onModeChange?: (mode: PortfolioAssetMode) => void;
  /** Shows the Simple / Strategic switch in the header. Defaults to true. */
  showModeToggle?: boolean;
  /** Asset classes that start collapsed in strategic mode. */
  defaultCollapsedClasses?: readonly AssetClass[];
  /** Optional heading override for embedding the collection in another pattern. */
  title?: React.ReactNode;
  /**
   * Optional supporting copy shown below the collection heading. Omit it for
   * the mode's default; pass `null` to hide it.
   */
  subtitle?: React.ReactNode;
  /**
   * Optional actions rendered in the header toolbar, on every viewport, next
   * to the Table mode control (desktop) or Table options button (mobile).
   */
  headerActions?: React.ReactNode;
  /** Opens the mobile Table options drawer on first render (Canvas stories). */
  defaultOptionsOpen?: boolean;
  /** Stated Strategic allocation shown instead of targets derived from the assets. */
  targets?: StrategicTargets;
  /** Portfolio-wide Drift band in percentage points; drift inside it reads as on target. */
  driftBandPp?: number;
  onEdit?: (asset: PortfolioAsset) => void;
  /** Edits an asset class's target weight (Strategic mode); adds `Edit {Class}` buttons. */
  onEditClass?: (assetClass: AssetClass) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}

const defaultSubtitles: Record<PortfolioAssetMode, string> = {
  simple: "Flat list of every asset",
  strategic: "Grouped by asset class",
};

/**
 * The single place that shows portfolio assets. One normalized model feeds
 * a dense table on wide screens and equivalent cards on small screens, in
 * either Simple (flat) or Strategic (grouped by asset class) mode. Sorting
 * and collapsed classes are shared by both presentations.
 */
export function PortfolioAssetCollection({
  assets,
  presentation = "detailed",
  mode: controlledMode,
  defaultMode = "simple",
  onModeChange,
  showModeToggle = true,
  defaultCollapsedClasses,
  title = "Investments",
  subtitle,
  headerActions,
  defaultOptionsOpen = false,
  targets,
  driftBandPp = DEFAULT_DRIFT_BAND_PP,
  onEdit,
  onEditClass,
  onMenu,
}: PortfolioAssetCollectionProps) {
  const headingId = React.useId();
  const [uncontrolledMode, setUncontrolledMode] =
    React.useState<PortfolioAssetMode>(defaultMode);
  const mode = controlledMode ?? uncontrolledMode;
  const view = usePortfolioAssetViewState(defaultCollapsedClasses, mode);
  const summary = React.useMemo(
    () => buildPortfolioAssetSummary(assets, targets),
    [assets, targets]
  );
  // Without assets there is nothing to order, so the options button is omitted.
  const showOptions = assets.length > 0;
  const shownSubtitle =
    subtitle === undefined ? defaultSubtitles[mode] : subtitle;

  const handleModeChange = (value: string) => {
    const next: PortfolioAssetMode =
      value === "strategic" ? "strategic" : "simple";
    // Each mode starts from the default ordering so the lists are predictable.
    view.resetSort();
    setUncontrolledMode(next);
    onModeChange?.(next);
  };

  return (
    <DriftBandProvider bandPp={driftBandPp}>
      <section
        aria-labelledby={headingId}
        className={`ds-asset-collection ds-asset-collection--${presentation}`}
      >
        <header className="ds-asset-collection__header">
          <div>
            <h2 className="ds-asset-collection__title" id={headingId}>
              {title}
            </h2>
            {shownSubtitle ? (
              <span className="ds-asset-collection__subtitle">
                {shownSubtitle}
              </span>
            ) : null}
          </div>
          <div className="ds-asset-collection__header-side">
            <span className="ds-asset-collection__count" aria-live="polite">
              {assets.length} {assets.length === 1 ? "asset" : "assets"}
            </span>
          </div>
          {headerActions || showOptions ? (
            <div className="ds-asset-collection__toolbar">
              {headerActions ? (
                <div className="ds-asset-collection__header-actions">
                  {headerActions}
                </div>
              ) : null}
              {showModeToggle ? (
                <div className="ds-asset-collection__mode">
                  <SegmentedControl
                    label="Table mode"
                    size="sm"
                    options={portfolioAssetModeOptions}
                    value={mode}
                    onValueChange={handleModeChange}
                  />
                </div>
              ) : null}
              {showOptions ? (
                <PortfolioAssetTableOptions
                  mode={mode}
                  onModeChange={handleModeChange}
                  showModeToggle={showModeToggle}
                  view={view}
                  defaultOpen={defaultOptionsOpen}
                />
              ) : null}
            </div>
          ) : null}
        </header>
        {assets.length === 0 ? (
          <EmptyState
            title="No portfolio assets yet"
            description="Add an asset to start tracking your allocation."
          />
        ) : (
          <>
            <div className="ds-asset-collection__cards">
              {mode === "strategic"
                ? sortAssetGroups(summary.groups, view.sort).map((group) => (
                    <PortfolioAssetGroupCard
                      key={group.assetClass}
                      group={group}
                      presentation={presentation}
                      expanded={
                        !view.collapsedClasses.includes(group.assetClass)
                      }
                      onToggle={() => view.onToggleClass(group.assetClass)}
                      onEdit={onEdit}
                      onEditClass={onEditClass}
                      onMenu={onMenu}
                    />
                  ))
                : sortAssetEntries(summary.entries, view.sort).map((entry) => (
                    <PortfolioAssetCard
                      key={entry.asset.id}
                      entry={entry}
                      presentation={presentation}
                      onEdit={onEdit}
                      onMenu={onMenu}
                    />
                  ))}
            </div>
            <div className="ds-asset-collection__table">
              <PortfolioAssetTable
                assets={assets}
                mode={mode}
                presentation={presentation}
                view={view}
                targets={targets}
                onEdit={onEdit}
                onEditClass={onEditClass}
                onMenu={onMenu}
              />
            </div>
          </>
        )}
      </section>
    </DriftBandProvider>
  );
}
