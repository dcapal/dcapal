import * as React from "react";
import {
  Button,
  Dialog,
  DialogCloseButton,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerTitle,
  Field,
  FieldGroup,
  FieldLabel,
  IconButton,
  InputWithSuffix,
  SegmentedControl,
} from "../ui";
import { X } from "lucide-react";

const MOBILE_QUERY = "(max-width: 48rem)";

/** Fee choices used by portfolio and asset fee controls. */
export type TransactionFeePolicy = "portfolio" | "zero" | "fixed" | "variable";

/** Presentation-only values submitted by the fee configuration form. */
export interface TransactionFeeConfigurationValues {
  policy: TransactionFeePolicy;
  maximumImpact: string;
  amount: string;
  feeRate: string;
  minFee: string;
  maxFee: string;
}

/** Props for the shared inline and responsive transaction-fee forms. */
export interface TransactionFeeConfigurationFormProps {
  policy: TransactionFeePolicy;
  onPolicyChange: (policy: TransactionFeePolicy) => void;
  maximumImpact: string;
  amount: string;
  feeRate: string;
  minFee: string;
  maxFee: string;
  onMaximumImpactChange: (value: string) => void;
  onAmountChange: (value: string) => void;
  onFeeRateChange: (value: string) => void;
  onMinFeeChange: (value: string) => void;
  onMaxFeeChange: (value: string) => void;
  /** Asset controls include the inherited portfolio policy in the editor. */
  includePortfolioDefault?: boolean;
  /** Portfolio-level values shown when the asset inherits the global policy. */
  portfolioValues?: Pick<
    TransactionFeeConfigurationValues,
    "maximumImpact" | "amount" | "feeRate" | "minFee" | "maxFee"
  >;
  /** A unique prefix keeps inline and portalled labels independent. */
  idPrefix?: string;
  className?: string;
}

function getIsMobileSnapshot() {
  return (
    typeof window !== "undefined" && window.matchMedia(MOBILE_QUERY).matches
  );
}

/** Subscribes to the responsive fee-overlay breakpoint without mounting both primitives. */
function subscribeToViewport(callback: () => void) {
  if (typeof window === "undefined") return () => undefined;
  const mediaQuery = window.matchMedia(MOBILE_QUERY);
  if (mediaQuery.addEventListener)
    mediaQuery.addEventListener("change", callback);
  else mediaQuery.addListener(callback);
  return () => {
    if (mediaQuery.removeEventListener)
      mediaQuery.removeEventListener("change", callback);
    else mediaQuery.removeListener(callback);
  };
}

/** Tracks the responsive overlay breakpoint without mounting both primitives. */
function useIsMobileViewport() {
  return React.useSyncExternalStore(
    subscribeToViewport,
    getIsMobileSnapshot,
    () => false
  );
}

/** Renders the legacy fee fields for the selected policy. */
export function TransactionFeeConfigurationForm({
  policy,
  onPolicyChange,
  maximumImpact,
  amount,
  feeRate,
  minFee,
  maxFee,
  onMaximumImpactChange,
  onAmountChange,
  onFeeRateChange,
  onMinFeeChange,
  onMaxFeeChange,
  includePortfolioDefault = false,
  portfolioValues,
  idPrefix = "transaction-fee",
  className,
}: TransactionFeeConfigurationFormProps) {
  const options = [
    ...(includePortfolioDefault
      ? [{ value: "portfolio", label: "Portfolio default" }]
      : []),
    { value: "zero", label: "Zero" },
    { value: "fixed", label: "Fixed" },
    { value: "variable", label: "Variable" },
  ] as const;
  const fieldId = (name: string) => `${idPrefix}-${name}`;
  const minFeeIsValid =
    !minFee ||
    !maxFee ||
    Number.isNaN(Number(minFee)) ||
    Number.isNaN(Number(maxFee)) ||
    Number(minFee) <= Number(maxFee);
  const inheritedValues = portfolioValues ?? {
    maximumImpact,
    amount,
    feeRate,
    minFee,
    maxFee,
  };
  return (
    <div className={className}>
      <SegmentedControl
        label="Transaction fee policy"
        value={policy}
        size="sm"
        onValueChange={(value) => onPolicyChange(value as TransactionFeePolicy)}
        options={options}
      />
      {policy === "portfolio" ? (
        <p className="ds-description ds-fee-configuration__inherited">
          Uses the configured portfolio transaction fees: maximum impact{" "}
          {inheritedValues.maximumImpact || "0"}% · amount €
          {inheritedValues.amount || "0.00"}.
        </p>
      ) : null}
      {policy === "zero" ? (
        <p className="ds-description ds-fee-configuration__zero">
          No transaction fees are charged.
        </p>
      ) : null}
      {policy === "fixed" ? (
        <FieldGroup className="ds-fee-configuration__fields">
          <Field orientation="inline">
            <FieldLabel htmlFor={fieldId("maximum-impact")}>
              Maximum impact
            </FieldLabel>
            <InputWithSuffix
              id={fieldId("maximum-impact")}
              inputMode="decimal"
              value={maximumImpact}
              onChange={(event) => onMaximumImpactChange(event.target.value)}
              suffix="%"
            />
          </Field>
          <Field orientation="inline">
            <FieldLabel htmlFor={fieldId("amount")}>Amount</FieldLabel>
            <InputWithSuffix
              id={fieldId("amount")}
              inputMode="decimal"
              value={amount}
              onChange={(event) => onAmountChange(event.target.value)}
              suffix="EUR"
            />
          </Field>
        </FieldGroup>
      ) : null}
      {policy === "variable" ? (
        <FieldGroup className="ds-fee-configuration__fields">
          <Field orientation="inline">
            <FieldLabel htmlFor={fieldId("maximum-impact")}>
              Maximum impact
            </FieldLabel>
            <InputWithSuffix
              id={fieldId("maximum-impact")}
              inputMode="decimal"
              value={maximumImpact}
              onChange={(event) => onMaximumImpactChange(event.target.value)}
              suffix="%"
            />
          </Field>
          <Field orientation="inline">
            <FieldLabel htmlFor={fieldId("fee-rate")}>
              Fee percentage
            </FieldLabel>
            <InputWithSuffix
              id={fieldId("fee-rate")}
              inputMode="decimal"
              value={feeRate}
              onChange={(event) => onFeeRateChange(event.target.value)}
              suffix="%"
            />
          </Field>
          <Field orientation="inline">
            <FieldLabel htmlFor={fieldId("min-fee")}>Min fee</FieldLabel>
            <InputWithSuffix
              id={fieldId("min-fee")}
              inputMode="decimal"
              value={minFee}
              aria-invalid={!minFeeIsValid}
              onChange={(event) => onMinFeeChange(event.target.value)}
              suffix="EUR"
            />
          </Field>
          <Field orientation="inline">
            <FieldLabel htmlFor={fieldId("max-fee")}>Max fee</FieldLabel>
            <InputWithSuffix
              id={fieldId("max-fee")}
              inputMode="decimal"
              value={maxFee}
              onChange={(event) => onMaxFeeChange(event.target.value)}
              suffix="EUR"
            />
          </Field>
          <p className="ds-description ds-fee-configuration__explanation">
            Variable fees are estimated from the asset&apos;s spread and
            expected market impact.
          </p>
          {!minFeeIsValid ? (
            <p className="ds-status-error" role="alert">
              Minimum fee must not exceed maximum fee.
            </p>
          ) : null}
        </FieldGroup>
      ) : null}
    </div>
  );
}

/** Props for the responsive transaction-fee editor overlay. */
export interface TransactionFeeConfigurationProps {
  open: boolean;
  policy: TransactionFeePolicy;
  onOpenChange: (open: boolean) => void;
  onPolicyChange?: (policy: TransactionFeePolicy) => void;
  onSave?: (values: TransactionFeeConfigurationValues) => void;
  finalFocus?: React.RefObject<HTMLElement | null>;
  initialMaximumImpact?: string;
  initialAmount?: string;
  initialFeeRate?: string;
  initialMinFee?: string;
  initialMaxFee?: string;
  /** Values inherited by Portfolio default, kept separate from asset overrides. */
  portfolioDefaults?: Pick<
    TransactionFeeConfigurationValues,
    "maximumImpact" | "amount" | "feeRate" | "minFee" | "maxFee"
  >;
  includePortfolioDefault?: boolean;
}

/** A responsive fee editor that uses one nested Dialog or Drawer at a time. */
export function TransactionFeeConfiguration({
  open,
  policy,
  onOpenChange,
  onPolicyChange,
  onSave,
  finalFocus,
  initialMaximumImpact = "0.5",
  initialAmount = "2.50",
  initialFeeRate = "0.25",
  initialMinFee = "1.00",
  initialMaxFee = "25.00",
  portfolioDefaults = {
    maximumImpact: "0.5",
    amount: "2.50",
    feeRate: "0.25",
    minFee: "1.00",
    maxFee: "25.00",
  },
  includePortfolioDefault = false,
}: TransactionFeeConfigurationProps) {
  const isMobile = useIsMobileViewport();
  const [draftPolicy, setDraftPolicy] = React.useState(policy);
  const [values, setValues] = React.useState({
    maximumImpact: initialMaximumImpact,
    amount: initialAmount,
    feeRate: initialFeeRate,
    minFee: initialMinFee,
    maxFee: initialMaxFee,
  });
  React.useEffect(() => {
    if (open) {
      setDraftPolicy(policy);
      setValues({
        maximumImpact: initialMaximumImpact,
        amount: initialAmount,
        feeRate: initialFeeRate,
        minFee: initialMinFee,
        maxFee: initialMaxFee,
      });
    }
  }, [
    initialAmount,
    initialFeeRate,
    initialMaximumImpact,
    initialMaxFee,
    initialMinFee,
    open,
    policy,
  ]);
  const feeRangeIsValid =
    !values.minFee ||
    !values.maxFee ||
    Number.isNaN(Number(values.minFee)) ||
    Number.isNaN(Number(values.maxFee)) ||
    Number(values.minFee) <= Number(values.maxFee);
  const save = () => {
    if (draftPolicy === "variable" && !feeRangeIsValid) return;
    onPolicyChange?.(draftPolicy);
    onSave?.({ policy: draftPolicy, ...values });
    onOpenChange(false);
  };
  const form = (
    <TransactionFeeConfigurationForm
      policy={draftPolicy}
      onPolicyChange={setDraftPolicy}
      {...values}
      onMaximumImpactChange={(maximumImpact) =>
        setValues((current) => ({ ...current, maximumImpact }))
      }
      onAmountChange={(amount) =>
        setValues((current) => ({ ...current, amount }))
      }
      onFeeRateChange={(feeRate) =>
        setValues((current) => ({ ...current, feeRate }))
      }
      onMinFeeChange={(minFee) =>
        setValues((current) => ({ ...current, minFee }))
      }
      onMaxFeeChange={(maxFee) =>
        setValues((current) => ({ ...current, maxFee }))
      }
      includePortfolioDefault={includePortfolioDefault}
      portfolioValues={portfolioDefaults}
      idPrefix="transaction-fee-overlay"
    />
  );
  if (isMobile) {
    return (
      <Drawer open={open} onOpenChange={onOpenChange}>
        <DrawerContent
          className="ds-fee-configuration ds-fees-dialog"
          layer="nested"
          finalFocus={finalFocus}
        >
          <DialogHeader className="ds-fee-configuration__header">
            <div>
              <DrawerTitle>Transaction fees</DrawerTitle>
              <DrawerDescription>
                Configure the fee assumptions used for this asset.
              </DrawerDescription>
            </div>
            <DrawerClose
              render={
                <IconButton label="Close transaction fees" variant="ghost">
                  <X aria-hidden strokeWidth={1.8} />
                </IconButton>
              }
            />
          </DialogHeader>
          {form}
          <DrawerFooter>
            <Button onClick={save}>Save</Button>
          </DrawerFooter>
        </DrawerContent>
      </Drawer>
    );
  }
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="ds-fee-configuration ds-fees-dialog"
        density="compact"
        layer="nested"
        finalFocus={finalFocus}
      >
        <DialogHeader className="ds-fee-configuration__header">
          <div>
            <DialogTitle>Transaction fees</DialogTitle>
            <DialogDescription>
              Configure the fee assumptions used for this asset.
            </DialogDescription>
          </div>
          <DialogCloseButton label="Close transaction fees" />
        </DialogHeader>
        {form}
        <DialogFooter>
          <Button onClick={save}>Save</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
