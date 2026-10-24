import type * as React from "react";
import type { PortfolioAsset } from "../../fixtures/portfolio";
import { EmptyState } from "../ui";
import { PortfolioAssetCard } from "./portfolio-asset-card";
import { PortfolioAssetTable } from "./portfolio-asset-table";

export interface PortfolioAssetCollectionProps {
  assets: readonly PortfolioAsset[];
  /** Optional actions rendered beside the collection heading on wide layouts. */
  headerActions?: React.ReactNode;
  onEdit?: (asset: PortfolioAsset) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}

/** The responsive contract: one normalized model, equivalent card and table labels/actions. */
export function PortfolioAssetCollection({
  assets,
  headerActions,
  onEdit,
  onMenu,
}: PortfolioAssetCollectionProps) {
  return (
    <section aria-label="Investments" className="ds-asset-collection">
      <header className="ds-asset-collection__header">
        <div>
          <h2 className="ds-asset-collection__title">Investments</h2>
          <span className="ds-asset-collection__subtitle">
            Grouped by individual asset
          </span>
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
              onEdit={onEdit}
              onMenu={onMenu}
            />
          </div>
        </>
      )}
    </section>
  );
}
