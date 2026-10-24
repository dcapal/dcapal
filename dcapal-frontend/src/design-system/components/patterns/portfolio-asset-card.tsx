import { MoreVertical, Pencil } from "lucide-react";
import { AssetTickerChip, IconButton } from "../ui";
import {
  assetClassLabels,
  formatCurrency,
  formatPercent,
  type PortfolioAsset,
} from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import { PortfolioDriftBadge } from "./portfolio-asset-drift-badge";
import {
  assetRegion,
  type PortfolioAssetEntry,
} from "./portfolio-asset-groups";

export interface PortfolioAssetCardProps {
  /** The asset with the weights derived from its market value. */
  entry: PortfolioAssetEntry;
  /**
   * Set when the card sits inside an asset-class card. A grouped card is a
   * compact row that shows the asset's share of its class instead of the
   * class and region.
   */
  grouped?: boolean;
  /** `detailed` adds quantity, price and cost basis, matching the desktop table. */
  presentation?: "detailed" | "overview";
  onEdit?: (asset: PortfolioAsset) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}

/**
 * The mobile counterpart of a portfolio asset table row. The name wraps beside
 * the drift badge; a second line carries the ticker chip, context and value.
 */
export function PortfolioAssetCard({
  entry,
  grouped = false,
  presentation = "detailed",
  onEdit,
  onMenu,
}: PortfolioAssetCardProps) {
  const { asset } = entry;
  return (
    <article
      className={cn("ds-asset-card", grouped && "ds-asset-card--grouped")}
      data-asset-class={asset.assetClass}
    >
      <div className="ds-asset-card__heading">
        <span className="ds-asset-card__name">{asset.name}</span>
        <div className="ds-asset-card__trailing">
          <PortfolioDriftBadge driftPp={entry.driftPp} />
          {onEdit ? (
            <IconButton
              label={`Edit ${asset.ticker}`}
              variant="ghost"
              size="sm"
              onClick={() => onEdit(asset)}
            >
              <Pencil size={16} aria-hidden />
            </IconButton>
          ) : null}
          {onMenu ? (
            <IconButton
              label={`More actions for ${asset.ticker}`}
              variant="ghost"
              size="sm"
              onClick={() => onMenu(asset)}
            >
              <MoreVertical size={16} aria-hidden />
            </IconButton>
          ) : null}
        </div>
      </div>
      <div className="ds-asset-card__line">
        <AssetTickerChip ticker={asset.ticker} />
        <span className="ds-asset-card__meta">
          {grouped
            ? `${formatPercent(entry.currentWeight)} → ${formatPercent(entry.targetWeight)} · ${formatPercent(entry.targetShareOfClass)} of class`
            : `${assetClassLabels[asset.assetClass]} · ${assetRegion(asset)}`}
        </span>
        <span className="ds-asset-card__value">
          {formatCurrency(asset.value, asset.currency)}
        </span>
      </div>
      {grouped ? null : (
        <div className="ds-asset-card__meta">
          {formatPercent(entry.currentWeight)} now ·{" "}
          {formatPercent(entry.targetWeight)} target
        </div>
      )}
      {presentation === "detailed" ? (
        <p className="ds-asset-card__details">
          Quantity {asset.quantity ? asset.quantity.toFixed(2) : "—"} · Current
          price {formatCurrency(asset.price, asset.currency)} · Average cost
          basis{" "}
          {asset.averageCostBasis === undefined
            ? "—"
            : formatCurrency(asset.averageCostBasis, asset.currency)}
        </p>
      ) : null}
    </article>
  );
}
