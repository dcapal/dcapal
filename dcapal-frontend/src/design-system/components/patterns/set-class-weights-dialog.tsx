import * as React from "react";
import { Button, Alert, Badge, InputWithSuffix } from "../ui";
import {
  assetClassLabels,
  type AssetClass,
  type PortfolioAsset,
} from "../../fixtures/portfolio";
import { formatDeviation } from "./invest-format";
import { assetClassOrder } from "./portfolio-asset-groups";
import { ResponsiveOverlay } from "./responsive-overlay";
import { weightInputA11y, WeightInputError } from "./weight-input-error";
import {
  checkStrategicTotals,
  classTargetsFromAssets,
  currentMixFromAssets,
  describeWeightTotal,
  emptyClassWeights,
  readWeightInput,
  sharesFromAssets,
  sumWeights,
  type AssetShares,
  type ClassWeights,
} from "./strategic-allocation";

export interface SetClassWeightsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The Portfolio Assets, used for the starters, counts and current weights. */
  assets: readonly PortfolioAsset[];
  /** Class weights the dialog opens with. Defaults to the weights implied by the asset targets. */
  initialWeights?: ClassWeights;
  /**
   * The asset shares that will really be stored on save, so the warning here
   * equals the Invest gate. Defaults to shares derived from the asset targets.
   */
  shares?: AssetShares;
  /** Element focus returns to when the dialog closes, such as the button that opened it. */
  finalFocus?: React.ComponentProps<typeof ResponsiveOverlay>["finalFocus"];
  /** Called with the chosen class weights. Saving is always allowed, balanced or not. */
  onSave: (weights: ClassWeights) => void;
  /** Small label above the title. */
  eyebrow?: React.ReactNode;
  /** Label of the save button. */
  saveLabel?: string;
}

/** Text of a weight for an input field. */
function inputText(value: number): string {
  return String(Number(value.toFixed(2)));
}

/**
 * Sets the asset-class target weights of every class at once, in one table
 * with a stacked preview bar, current weights and gaps. Save is always
 * enabled; totals that miss 100% only produce a note, because Invest stays
 * unavailable until they balance. Nothing is redistributed automatically.
 */
export function SetClassWeightsDialog({
  open,
  onOpenChange,
  assets,
  initialWeights,
  shares,
  finalFocus,
  onSave,
  eyebrow = "Switching to Strategic",
  saveLabel = "Save and switch to Strategic",
}: SetClassWeightsDialogProps) {
  const targets = React.useMemo(() => classTargetsFromAssets(assets), [assets]);
  const mix = React.useMemo(() => currentMixFromAssets(assets), [assets]);
  const [draft, setDraft] = React.useState<Record<AssetClass, string>>(() =>
    toDraft(initialWeights ?? targets)
  );
  React.useEffect(() => {
    if (open) setDraft(toDraft(initialWeights ?? targets));
    // Re-seed only when the dialog opens.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const readings = Object.fromEntries(
    assetClassOrder.map((c) => [c, readWeightInput(draft[c])])
  ) as Record<AssetClass, ReturnType<typeof readWeightInput>>;
  const weights = Object.fromEntries(
    assetClassOrder.map((c) => [c, readings[c].value])
  ) as ClassWeights;
  const invalidLabels = assetClassOrder
    .filter((c) => readings[c].error)
    .map((c) => assetClassLabels[c]);
  const total = describeWeightTotal(sumWeights(Object.values(weights)));
  const check = checkStrategicTotals(assets, {
    classTargets: weights,
    shares: shares ?? sharesFromAssets(assets),
  });
  const emptyClassNotes = check.issues.filter(
    (issue) => issue.scope === "class-without-assets"
  );
  const counts = (assetClass: AssetClass) =>
    assets.filter((asset) => asset.assetClass === assetClass).length;
  const bar = assetClassOrder
    .map(
      (assetClass) =>
        `${assetClassLabels[assetClass]} ${weights[assetClass].toFixed(1)}%`
    )
    .join(", ");

  return (
    <ResponsiveOverlay
      open={open}
      onOpenChange={onOpenChange}
      className="ds-class-weights-dialog"
      finalFocus={finalFocus}
      closeLabel="class weights"
      eyebrow={eyebrow}
      title="Set your asset class weights"
      description="Decide how much of the portfolio each class should be, all in one place. Each asset keeps its share of its class, so nothing else changes until you edit it."
      footer={
        <>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={() => {
              onSave(weights);
              onOpenChange(false);
            }}
          >
            {saveLabel}
          </Button>
        </>
      }
    >
      <div className="ds-class-weights__starters">
        <Button variant="link" onClick={() => setDraft(toDraft(targets))}>
          Use my current targets
        </Button>
        <Button variant="link" onClick={() => setDraft(toDraft(mix))}>
          Use my current mix
        </Button>
        <Button
          variant="link"
          onClick={() => setDraft(toDraft(emptyClassWeights()))}
        >
          Clear
        </Button>
      </div>
      <div
        className="ds-class-weights__bar"
        role="img"
        aria-label={`Chosen class weights: ${bar}`}
      >
        {assetClassOrder.map((assetClass) => (
          <span
            key={assetClass}
            aria-hidden
            data-asset-class={assetClass}
            style={{ width: `${Math.min(weights[assetClass], 100)}%` }}
          />
        ))}
      </div>
      <div
        className="ds-class-weights"
        role="table"
        aria-label="Asset class weights"
      >
        <div
          className="ds-class-weights__row ds-class-weights__row--head"
          role="row"
        >
          <span role="columnheader">Asset class</span>
          <span role="columnheader">Weight</span>
          <span role="columnheader">Now · gap</span>
        </div>
        {assetClassOrder.map((assetClass) => {
          const label = assetClassLabels[assetClass];
          const count = counts(assetClass);
          return (
            <div
              className="ds-class-weights__row"
              role="row"
              key={assetClass}
              data-asset-class={assetClass}
            >
              <span role="cell" className="ds-class-weights__name">
                <strong>{label}</strong>
                <span>
                  {count === 0
                    ? "No assets yet"
                    : `${count} ${count === 1 ? "asset" : "assets"}`}
                </span>
              </span>
              <span role="cell">
                <InputWithSuffix
                  aria-label={`${label} class weight`}
                  inputMode="decimal"
                  suffix="%"
                  value={draft[assetClass]}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      [assetClass]: event.target.value,
                    }))
                  }
                  {...weightInputA11y(
                    readings[assetClass].error,
                    `class-weight-error-${assetClass}`
                  )}
                />
                <WeightInputError
                  id={`class-weight-error-${assetClass}`}
                  error={readings[assetClass].error}
                />
              </span>
              <span role="cell" className="ds-class-weights__now">
                {mix[assetClass].toFixed(1)}% ·{" "}
                <strong>
                  {formatDeviation(mix[assetClass] - weights[assetClass])}
                </strong>
              </span>
            </div>
          );
        })}
        <div className="ds-weight-total" role="row">
          <span role="rowheader">Total</span>
          <span role="cell">
            <strong>{total.total.toFixed(1)}%</strong>
            <Badge variant={total.state === "balanced" ? "success" : "warning"}>
              {total.label}
            </Badge>
          </span>
        </div>
      </div>
      {!check.ok || invalidLabels.length > 0 ? (
        <Alert variant="warning">
          <span>
            {invalidLabels.length > 0
              ? `${invalidLabels.join(", ")} ${invalidLabels.length === 1 ? "isn't" : "aren't"} a number from 0 to 100 and count${invalidLabels.length === 1 ? "s" : ""} as 0. `
              : ""}
            {!check.ok
              ? total.state !== "balanced"
                ? "Class weights don't total 100%. You can save and finish later, but Invest stays unavailable until they do. "
                : "You can save and finish later, but Invest stays unavailable until this is fixed. "
              : "Fix the entry before saving, or save it as 0. "}
            {emptyClassNotes
              .map((issue) =>
                issue.scope === "class-without-assets"
                  ? `${assetClassLabels[issue.assetClass]} has a weight but no assets.`
                  : ""
              )
              .join(" ")}
          </span>
        </Alert>
      ) : null}
    </ResponsiveOverlay>
  );
}

function toDraft(weights: ClassWeights): Record<AssetClass, string> {
  return Object.fromEntries(
    assetClassOrder.map((c) => [c, inputText(weights[c])])
  ) as Record<AssetClass, string>;
}
