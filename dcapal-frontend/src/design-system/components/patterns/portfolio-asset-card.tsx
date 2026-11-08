import { MoreVertical, Pencil } from "lucide-react";
import * as React from "react";
import { AssetTickerChip, IconButton } from "../ui";
import {
  assetClassLabels,
  formatCurrency,
  formatPercent,
  type AssetClass,
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

export interface AssetCardShellProps {
  /** Colours the card's left rail. */
  assetClass: AssetClass;
  /** Set when the card sits inside an asset-class card. */
  grouped?: boolean;
  /** Renders an `li` when the card sits in a list; an `article` otherwise. */
  as?: "article" | "li";
  name: React.ReactNode;
  /** Badges and buttons at the end of the heading line. */
  trailing?: React.ReactNode;
  ticker: string;
  /** Muted text after the ticker chip. */
  meta?: React.ReactNode;
  /** Figure at the end of the chip line. */
  value?: React.ReactNode;
  /** Extra lines below the chip line. */
  children?: React.ReactNode;
  "data-row-kind"?: string;
}

/**
 * The presentational layout of every asset card: name and trailing controls,
 * then the ticker chip with its meta and value. The portfolio asset card and
 * the Invest result's change card both render it.
 */
export function AssetCardShell({
  assetClass,
  grouped = false,
  as: Tag = "article",
  name,
  trailing,
  ticker,
  meta,
  value,
  children,
  "data-row-kind": rowKind,
}: AssetCardShellProps) {
  return (
    <Tag
      className={cn("ds-asset-card", grouped && "ds-asset-card--grouped")}
      data-asset-class={assetClass}
      data-row-kind={rowKind}
    >
      <div className="ds-asset-card__heading">
        <span className="ds-asset-card__name">{name}</span>
        <div className="ds-asset-card__trailing">{trailing}</div>
      </div>
      <div className="ds-asset-card__line">
        <AssetTickerChip ticker={ticker} />
        <span className="ds-asset-card__meta">{meta}</span>
        {value === undefined ? null : (
          <span className="ds-asset-card__value">{value}</span>
        )}
      </div>
      {children}
    </Tag>
  );
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
    <AssetCardShell
      assetClass={asset.assetClass}
      grouped={grouped}
      name={asset.name}
      ticker={asset.ticker}
      trailing={
        <>
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
        </>
      }
      meta={
        grouped
          ? `${formatPercent(entry.currentWeight)} → ${formatPercent(entry.targetWeight)} · ${formatPercent(entry.targetShareOfClass)} of class`
          : `${assetClassLabels[asset.assetClass]} · ${assetRegion(asset)}`
      }
      value={formatCurrency(asset.value, asset.currency)}
    >
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
    </AssetCardShell>
  );
}
