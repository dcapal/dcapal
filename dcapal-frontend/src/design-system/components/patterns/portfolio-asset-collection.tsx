import * as React from "react";
import type { PortfolioAsset } from "../../fixtures/portfolio";
import { EmptyState } from "../ui";
import { PortfolioAssetCard } from "./portfolio-asset-card";
import { PortfolioAssetTable } from "./portfolio-asset-table";

export interface PortfolioAssetCollectionProps {
  assets: readonly PortfolioAsset[];
  /** Selects a focused holdings summary or the full asset-management table. */
  presentation?: "detailed" | "overview";
  /** Optional heading override for embedding the collection in another pattern. */
  title?: React.ReactNode;
  /** Optional supporting copy shown below the collection heading. */
  subtitle?: React.ReactNode;
  /** Optional actions rendered beside the collection heading on wide layouts. */
  headerActions?: React.ReactNode;
  onEdit?: (asset: PortfolioAsset) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}

/** The responsive contract: one normalized model, equivalent card and table labels/actions. */
export function PortfolioAssetCollection({
  assets,
  presentation = "detailed",
  title = "Investments",
  subtitle = "Grouped by individual asset",
  headerActions,
  onEdit,
  onMenu,
}: PortfolioAssetCollectionProps) {
  const headingId = React.useId();

  return (
    <section
      aria-labelledby={headingId}
      className={`ds-asset-collection ds-asset-collection--${presentation}`}
    >
      <header className="ds-asset-collection__header">
        <div>
          <h2 className="ds-asset-collection__title" id={headingId}>
            {title}
          </h2>
          {subtitle ? (
            <span className="ds-asset-collection__subtitle">{subtitle}</span>
          ) : null}
        </div>
        <div className="ds-asset-collection__header-side">
          {headerActions ? (
            <div className="ds-asset-collection__header-actions">
              {headerActions}
            </div>
          ) : null}
          <span className="ds-asset-collection__count" aria-live="polite">
            {assets.length} {assets.length === 1 ? "asset" : "assets"}
          </span>
        </div>
      </header>
      {assets.length === 0 ? (
        <EmptyState
          title="No portfolio assets yet"
          description="Add an asset to start tracking your allocation."
        />
      ) : (
        <>
          <div className="ds-asset-collection__cards">
            {assets.map((asset) => (
              <PortfolioAssetCard
                key={asset.id}
                asset={asset}
                onEdit={onEdit}
                onMenu={onMenu}
              />
            ))}
          </div>
          <div className="ds-asset-collection__table">
            <PortfolioAssetTable
              assets={assets}
              presentation={presentation}
              onEdit={onEdit}
              onMenu={onMenu}
            />
          </div>
        </>
      )}
    </section>
  );
}
