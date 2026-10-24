import * as React from "react";
import {
  Alert,
  AssetTickerChip,
  Badge,
  Button,
  Field,
  FieldGroup,
  FieldLabel,
  Input,
  InputWithSuffix,
  Separator,
} from "../ui";
import {
  assetClassLabels,
  formatAssetClass,
  formatPercent,
  type PortfolioAsset,
} from "../../fixtures/portfolio";
import type { AllocationMode } from "./invest-types";
import { driftState } from "./portfolio-asset-groups";
import { ResponsiveOverlay } from "./responsive-overlay";
import { weightInputA11y, WeightInputError } from "./weight-input-error";
import {
  readWeightInput,
  previewShareChange,
  type StrategicAllocation,
} from "./strategic-allocation";

/** What the investor changed in the asset's target weight. */
export type EditedAssetWeight =
  | { method: "simple"; absoluteWeight: number }
  | { method: "strategic"; share: number };

export interface EditPortfolioAssetDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  asset: PortfolioAsset;
  /** Every Portfolio Asset; the siblings of the asset's class are listed in Strategic mode. */
  assets: readonly PortfolioAsset[];
  /** The Portfolio's allocation method. */
  method: AllocationMode;
  allocation: StrategicAllocation;
  /** Transaction fee summary and editor trigger, owned by the caller. */
  feeSummary: React.ReactNode;
  /** Shows a Remove asset action when set. */
  onRemove?: () => void;
  /** Called with the edited target weight when the investor saves. */
  onSave: (weight: EditedAssetWeight) => void;
  finalFocus?: React.ComponentProps<typeof ResponsiveOverlay>["finalFocus"];
}

/**
 * The asset edit form. Simple mode edits the single absolute target weight.
 * Strategic mode edits the asset's share of its class (relative target
 * weight) and previews the resulting weight in the portfolio and every
 * sibling's share; shares that miss 100% only produce a note.
 */
export function EditPortfolioAssetDialog({
  open,
  onOpenChange,
  asset,
  assets,
  method,
  allocation,
  feeSummary,
  onRemove,
  onSave,
  finalFocus,
}: EditPortfolioAssetDialogProps) {
  const strategic = method === "strategic";
  const initial = strategic
    ? (allocation.shares[asset.id] ?? 0)
    : asset.targetWeight;
  // Shown to one decimal; an untouched field keeps the exact stored value so
  // a share such as 46.15 is not nudged by rounding.
  const initialText = String(Number(initial.toFixed(1)));
  const [draft, setDraft] = React.useState(initialText);
  React.useEffect(() => {
    if (open) setDraft(initialText);
    // Re-seed only when the dialog opens for an asset.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, asset.id]);

  const className = assetClassLabels[asset.assetClass];
  const { value: weight, error: weightError } =
    draft === initialText
      ? { value: initial, error: null }
      : readWeightInput(draft);
  const preview = strategic
    ? previewShareChange(assets, allocation, asset.id, weight)
    : null;

  return (
    <ResponsiveOverlay
      open={open}
      onOpenChange={onOpenChange}
      className="ds-edit-asset-dialog"
      closeLabel="edit asset"
      finalFocus={finalFocus}
      title={`Edit ${asset.ticker}`}
      description="Review and edit asset details."
      footer={
        <>
          {onRemove ? (
            <Button variant="destructive" type="button" onClick={onRemove}>
              Remove asset
            </Button>
          ) : null}
          <Button
            onClick={() => {
              onSave(
                strategic
                  ? { method: "strategic", share: weight }
                  : { method: "simple", absoluteWeight: weight }
              );
              onOpenChange(false);
            }}
          >
            Save changes
          </Button>
        </>
      }
    >
      <div className="ds-asset-dialog-identity">
        <span
          className={`ds-asset-card__dot ds-asset-card__dot--${asset.assetClass}`}
          aria-hidden
        />
        <div>
          <strong>{asset.name}</strong>
          <span>{formatAssetClass(asset)}</span>
        </div>
        <AssetTickerChip ticker={asset.ticker} />
      </div>
      <Separator />
      <FieldGroup>
        <Field orientation="inline">
          <FieldLabel htmlFor="edit-quantity">Quantity</FieldLabel>
          <Input id="edit-quantity" defaultValue={String(asset.quantity)} />
        </Field>
        <Field orientation="inline">
          <FieldLabel htmlFor="edit-basis">Basis cost / PMC</FieldLabel>
          <InputWithSuffix
            id="edit-basis"
            defaultValue={asset.averageCostBasis?.toFixed(2) ?? "0.00"}
            suffix={asset.currency ?? "EUR"}
          />
        </Field>
        {strategic && preview ? (
          <section
            className="ds-weight-section"
            aria-label={`${className} share editor`}
          >
            <div className="ds-weight-section__head">
              <div>
                <h3>
                  <label htmlFor="edit-share">Share of {className}</label>
                </h3>
                <span>
                  {className} target is{" "}
                  {formatPercent(allocation.classTargets[asset.assetClass])} of
                  the portfolio. Change it from the {className} row.
                </span>
              </div>
              <InputWithSuffix
                id="edit-share"
                inputMode="decimal"
                suffix="%"
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                {...weightInputA11y(weightError, "edit-share-error")}
              />
              <WeightInputError id="edit-share-error" error={weightError} />
            </div>
            <ul className="ds-weight-list ds-weight-list--shares">
              {preview.rows.map((row) => (
                <li key={row.assetId} data-edited={row.edited || undefined}>
                  <AssetTickerChip ticker={row.ticker} />
                  <span className="ds-weight-list__text">
                    <span className="ds-weight-list__change">
                      <span className="ds-muted">
                        {formatPercent(row.before)}
                      </span>{" "}
                      → {formatPercent(row.after)}
                    </span>{" "}
                    <span className="ds-muted ds-weight-list__absolute">
                      = {formatPercent(row.absolute)} of portfolio
                    </span>
                  </span>
                </li>
              ))}
            </ul>
            <div className="ds-weight-total">
              <span>Shares total</span>
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
                    ? "The share isn't a number from 0 to 100, so it counts as 0. "
                    : ""}
                  {className} shares don&apos;t total 100%. You can save, but
                  Invest stays unavailable until they do.
                </span>
              </Alert>
            ) : null}
          </section>
        ) : (
          <Field orientation="inline">
            <FieldLabel htmlFor="edit-target">Target weight</FieldLabel>
            <InputWithSuffix
              id="edit-target"
              inputMode="decimal"
              suffix="%"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              {...weightInputA11y(weightError, "edit-target-error")}
            />
            <WeightInputError id="edit-target-error" error={weightError} />
          </Field>
        )}
        <Field orientation="inline">
          <FieldLabel>Current weight</FieldLabel>
          <span className="ds-value-readonly">
            {asset.currentWeight.toFixed(1)}%
          </span>
        </Field>
        <Field orientation="inline">
          <FieldLabel>Drift</FieldLabel>
          <span
            className={`ds-value-readonly ds-status-${
              driftState(asset.driftPp) === "over"
                ? "warning"
                : driftState(asset.driftPp) === "under"
                  ? "primary"
                  : "success"
            }`}
          >
            {asset.driftPp > 0 ? "+" : ""}
            {asset.driftPp.toFixed(1)} pp
          </span>
        </Field>
        <Field>
          <FieldLabel>Transaction fees</FieldLabel>
          {feeSummary}
        </Field>
      </FieldGroup>
    </ResponsiveOverlay>
  );
}
