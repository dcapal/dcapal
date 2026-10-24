import * as React from "react";
import { ChevronDown, Pencil } from "lucide-react";
import { IconButton } from "../ui";
import {
  formatCurrency,
  formatPercent,
  type AssetClass,
  type PortfolioAsset,
} from "../../fixtures/portfolio";
import { PortfolioAssetCard } from "./portfolio-asset-card";
import { PortfolioDriftBadge } from "./portfolio-asset-drift-badge";
import type { PortfolioAssetClassGroup } from "./portfolio-asset-groups";

export interface PortfolioAssetGroupCardProps {
  group: PortfolioAssetClassGroup;
  expanded: boolean;
  presentation?: "detailed" | "overview";
  onToggle: () => void;
  onEdit?: (asset: PortfolioAsset) => void;
  /** Edits the class target weight; shows an `Edit {Class}` button in the header. */
  onEditClass?: (assetClass: AssetClass) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}

/**
 * Mobile card for one asset class in strategic mode: a collapsible header with
 * the class aggregates above compact rows for its assets.
 */
export function PortfolioAssetGroupCard({
  group,
  expanded,
  presentation,
  onToggle,
  onEdit,
  onEditClass,
  onMenu,
}: PortfolioAssetGroupCardProps) {
  const count = group.entries.length;
  const listId = React.useId();
  return (
    <section
      className="ds-asset-group-card"
      aria-label={group.label}
      data-asset-class={group.assetClass}
    >
      <div className="ds-asset-group-card__header">
        <div className="ds-asset-group-card__title-row">
          <button
            type="button"
            className="ds-asset-group-card__toggle"
            aria-expanded={expanded}
            aria-controls={listId}
            aria-label={`${expanded ? "Collapse" : "Expand"} ${group.label}`}
            onClick={onToggle}
          >
            <ChevronDown size={16} aria-hidden />
            <span className="ds-asset-group-card__name">{group.label}</span>
            <span className="ds-asset-group-card__count">
              {count} {count === 1 ? "asset" : "assets"}
            </span>
          </button>
          <PortfolioDriftBadge driftPp={group.driftPp} showStatus />
          {onEditClass ? (
            <IconButton
              label={`Edit ${group.label}`}
              variant="ghost"
              size="sm"
              onClick={() => onEditClass(group.assetClass)}
            >
              <Pencil size={16} aria-hidden />
            </IconButton>
          ) : null}
        </div>
        <dl className="ds-asset-group-card__stats">
          <div>
            <dt>Value</dt>
            <dd>{formatCurrency(group.value, group.currency)}</dd>
          </div>
          <div>
            <dt>Current / target</dt>
            <dd>
              {formatPercent(group.currentWeight)} /{" "}
              {formatPercent(group.targetWeight)}
            </dd>
          </div>
          <div>
            <dt>Invested</dt>
            <dd>
              {group.invested === null
                ? "—"
                : formatCurrency(group.invested, group.currency)}
            </dd>
          </div>
        </dl>
      </div>
      <div id={listId} hidden={!expanded}>
        {expanded
          ? group.entries.map((entry) => (
              <PortfolioAssetCard
                key={entry.asset.id}
                entry={entry}
                grouped
                presentation={presentation}
                onEdit={onEdit}
                onMenu={onMenu}
              />
            ))
          : null}
      </div>
    </section>
  );
}
