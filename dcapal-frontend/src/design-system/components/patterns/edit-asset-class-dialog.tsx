import * as React from "react";
import { Alert, AssetTickerChip, Badge, Button, InputWithSuffix } from "../ui";
import {
  assetClassLabels,
  formatPercent,
  type AssetClass,
  type PortfolioAsset,
} from "../../fixtures/portfolio";
import { assetClassOrder } from "./portfolio-asset-groups";
import { ResponsiveOverlay } from "./responsive-overlay";
import { weightInputA11y, WeightInputError } from "./weight-input-error";
import {
  readWeightInput,
  previewClassChange,
  sumWeights,
  type StrategicAllocation,
} from "./strategic-allocation";

export interface EditAssetClassDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  assetClass: AssetClass;
  /** Every Portfolio Asset, used for the class list and the read-only shares. */
  assets: readonly PortfolioAsset[];
  allocation: StrategicAllocation;
  /** Element focus returns to when the dialog closes, such as the class edit icon. */
  finalFocus?: React.ComponentProps<typeof ResponsiveOverlay>["finalFocus"];
  /** Called with the new asset-class target weight; saving is always allowed. */
  onSave: (weight: number) => void;
}

/**
 * Edits one asset-class target weight. Only the class weight changes: asset
 * shares are shown read-only and are edited from each asset. The class totals
 * show whether all class targets still reach 100%; Save is always enabled and
 * nothing else is adjusted automatically.
 */
export function EditAssetClassDialog({
  open,
  onOpenChange,
  assetClass,
  assets,
  allocation,
  finalFocus,
  onSave,
}: EditAssetClassDialogProps) {
  const label = assetClassLabels[assetClass];
  const current = allocation.classTargets[assetClass];
  const [draft, setDraft] = React.useState(String(Number(current.toFixed(2))));
  React.useEffect(() => {
    if (open) setDraft(String(Number(current.toFixed(2))));
    // Re-seed only when the dialog opens.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, assetClass]);

  const { value: weight, error: weightError } = readWeightInput(draft);
  const preview = previewClassChange(allocation, assetClass, weight);
  const members = assets.filter((asset) => asset.assetClass === assetClass);
  const nowOf = (c: AssetClass) =>
    sumWeights(
      assets.filter((a) => a.assetClass === c).map((a) => a.currentWeight)
    );
  const listed = assetClassOrder.filter(
    (c) =>
      c === assetClass ||
      allocation.classTargets[c] > 0 ||
      assets.some((a) => a.assetClass === c)
  );

  return (
    <ResponsiveOverlay
      open={open}
      onOpenChange={onOpenChange}
      className="ds-class-weights-dialog"
      finalFocus={finalFocus}
      closeLabel={`edit ${label}`}
      title={
        <span className="ds-class-dialog-title" data-asset-class={assetClass}>
          <span className="ds-asset-class-dot" aria-hidden />
          Edit {label}
        </span>
      }
      description={`Change how much of the portfolio ${label} should be. Each asset's share of the class stays as is; to change a share, edit the asset itself.`}
      footer={
        <>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={() => {
              onSave(weight);
              onOpenChange(false);
            }}
          >
            Save
          </Button>
        </>
      }
    >
      <section className="ds-weight-section" aria-label="Class weight">
        <div className="ds-weight-section__head">
          <div>
            <h3>Class weight</h3>
            <span>Share of the whole portfolio</span>
          </div>
          <InputWithSuffix
            aria-label={`${label} class weight`}
            inputMode="decimal"
            suffix="%"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            {...weightInputA11y(weightError, "edit-class-weight-error")}
          />
          <WeightInputError id="edit-class-weight-error" error={weightError} />
        </div>
        <ul className="ds-weight-list">
          {listed.map((c) => (
            <li key={c} data-asset-class={c}>
              <span>
                <span className="ds-asset-class-dot" aria-hidden />
                {assetClassLabels[c]}
              </span>
              <span>
                <span className="ds-muted">
                  {formatPercent(allocation.classTargets[c])}
                </span>{" "}
                → <strong>{formatPercent(preview.classTargets[c])}</strong>{" "}
                <span className="ds-muted">
                  · now {formatPercent(nowOf(c))}
                </span>
              </span>
            </li>
          ))}
        </ul>
        <div className="ds-weight-total">
          <span>Class targets total</span>
          <span>
            <strong>{preview.total.total.toFixed(1)}%</strong>
            <Badge
              variant={
                preview.total.state === "balanced" ? "success" : "warning"
              }
            >
              {preview.total.label}
            </Badge>
          </span>
        </div>
        {preview.total.state !== "balanced" || weightError ? (
          <Alert variant="warning">
            <span>
              {weightError
                ? `${label} weight isn't a number from 0 to 100, so it counts as 0. `
                : ""}
              Class targets don&apos;t total 100%. You can save, but Invest
              stays unavailable until they do.
            </span>
          </Alert>
        ) : null}
      </section>
      <section
        className="ds-weight-section"
        aria-label={`Asset shares in ${label}`}
      >
        <div className="ds-weight-section__head">
          <div>
            <h3>Asset shares in {label}</h3>
            <span>
              Edit from each asset. Weight in portfolio = class weight × share.
            </span>
          </div>
        </div>
        {members.length === 0 ? (
          <p className="ds-muted">No assets in {label} yet.</p>
        ) : (
          <ul className="ds-weight-list ds-weight-list--shares">
            {members.map((asset) => {
              const share = allocation.shares[asset.id] ?? 0;
              return (
                <li key={asset.id}>
                  <span className="ds-weight-list__asset">
                    <AssetTickerChip ticker={asset.ticker} />
                    <span className="ds-muted">{asset.name}</span>
                  </span>
                  <span>
                    <strong>{formatPercent(share)}</strong> of class ={" "}
                    <strong>{formatPercent((weight * share) / 100)}</strong> of
                    portfolio
                  </span>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </ResponsiveOverlay>
  );
}
