import * as React from "react";
import { MoreVertical, Pencil } from "lucide-react";
import { Card, CardContent, IconButton } from "../ui";
import {
  formatAssetClass,
  formatCurrency,
  formatDrift,
  formatPercent,
  type PortfolioAsset,
} from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";

export interface PortfolioAssetCardProps {
  asset: PortfolioAsset;
  onEdit?: (asset: PortfolioAsset) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}
export function PortfolioAssetCard({
  asset,
  onEdit,
  onMenu,
}: PortfolioAssetCardProps) {
  const driftVariant =
    asset.driftPp > 1 ? "warning" : asset.driftPp < -1 ? "primary" : "success";
  return (
    <Card className="ds-asset-card">
      <CardContent>
        <div className="ds-asset-card__heading">
          <span
            className={cn(
              "ds-asset-card__dot",
              `ds-asset-card__dot--${asset.assetClass}`
            )}
            aria-hidden
          />{" "}
          <div className="ds-asset-card__identity">
            <div className="ds-asset-card__title-row">
              <span className="ds-asset-card__name">{asset.name}</span>
              <span className="ds-asset-card__ticker">
                {` · ${asset.ticker}`}
              </span>
            </div>
            <span className="ds-asset-card__meta">
              {formatAssetClass(asset)}
            </span>
          </div>
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
        <div className="ds-asset-card__stats">
          <Stat
            label="Quantity"
            value={asset.quantity ? asset.quantity.toFixed(2) : "—"}
          />
          <Stat
            label="Current price"
            value={formatCurrency(asset.price, asset.currency)}
          />
          <Stat
            label="Average cost basis"
            value={
              asset.averageCostBasis === undefined
                ? "—"
                : formatCurrency(asset.averageCostBasis, asset.currency)
            }
          />
          <Stat
            label="Current value"
            value={formatCurrency(asset.value, asset.currency)}
          />
          <Stat
            label="Target weight"
            value={formatPercent(asset.targetWeight)}
          />
          <Stat
            label="Current weight"
            value={formatPercent(asset.currentWeight)}
          />
          <Stat
            label="Drift"
            value={formatDrift(asset.driftPp)}
            valueVariant={driftVariant}
          />
        </div>
      </CardContent>
    </Card>
  );
}

function Stat({
  label,
  value,
  valueVariant,
}: {
  label: string;
  value: string;
  valueVariant?: "primary" | "success" | "warning";
}) {
  return (
    <div className="ds-asset-card__stat">
      <span className="ds-asset-card__stat-label">{label}</span>
      <span
        className={cn(
          "ds-asset-card__stat-value",
          valueVariant && `ds-asset-card__stat-value--${valueVariant}`
        )}
      >
        {value}
      </span>
    </div>
  );
}
