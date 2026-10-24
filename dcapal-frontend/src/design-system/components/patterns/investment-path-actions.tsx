import * as React from "react";
import { Button, Tooltip } from "../ui";
import { cn } from "../../lib/cn";

export interface InvestmentPathActionsProps {
  /** Called when the investor chooses the Add money path. */
  onAddMoney?: () => void;
  /** Called when the investor chooses the Rebalance path. */
  onRebalance?: () => void;
  /** Prevents choosing a path, for example while targets do not total 100%. */
  disabled?: boolean;
  className?: string;
}

/**
 * The two ways to act on a drift view: Add money and Rebalance. Each button
 * explains what the path does through a tooltip and an accessible description.
 * Add money is the primary action and Rebalance the secondary one. Place the
 * group in the page header, outside any card; on a narrow container it spans
 * the full width and splits evenly.
 */
export function InvestmentPathActions({
  onAddMoney,
  onRebalance,
  disabled,
  className,
}: InvestmentPathActionsProps) {
  const addId = React.useId();
  const rebalanceId = React.useId();
  const addCopy = "Buys only, nothing you own is sold.";
  const rebalanceCopy =
    "Buys and sells to match your targets. You can add new money too, or none.";

  return (
    <div
      role="group"
      aria-label="Investment paths"
      className={cn("ds-path-actions", className)}
    >
      <Tooltip label={addCopy}>
        <Button
          variant="primary"
          disabled={disabled}
          aria-describedby={addId}
          onClick={onAddMoney}
        >
          Add money
        </Button>
      </Tooltip>
      <span id={addId} className="ds-sr-only">
        {addCopy}
      </span>
      <Tooltip label={rebalanceCopy}>
        <Button
          variant="secondary"
          disabled={disabled}
          aria-describedby={rebalanceId}
          onClick={onRebalance}
        >
          Rebalance
        </Button>
      </Tooltip>
      <span id={rebalanceId} className="ds-sr-only">
        {rebalanceCopy}
      </span>
    </div>
  );
}
