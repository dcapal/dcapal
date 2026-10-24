import * as React from "react";
import { Badge, type BadgeVariant } from "../ui";
import { formatDeviation } from "./invest-format";
import {
  DEFAULT_DRIFT_BAND_PP,
  driftState,
  driftStateLabels,
} from "./portfolio-asset-groups";

const DriftBandContext = React.createContext(DEFAULT_DRIFT_BAND_PP);

/**
 * Sets the Portfolio-wide Drift band, in percentage points, for every
 * {@link PortfolioDriftBadge} inside, so cards, tables and group rows agree.
 */
export function DriftBandProvider({
  bandPp,
  children,
}: {
  bandPp: number;
  children: React.ReactNode;
}) {
  return (
    <DriftBandContext.Provider value={bandPp}>
      {children}
    </DriftBandContext.Provider>
  );
}

const driftVariants: Record<ReturnType<typeof driftState>, BadgeVariant> = {
  over: "warning",
  under: "primary",
  "on-target": "success",
};

export interface PortfolioDriftBadgeProps {
  /** Current weight minus target weight, in percentage points. */
  driftPp: number;
  /** Adds the spelled-out status to the visible badge, for compact cards. */
  showStatus?: boolean;
}

/**
 * Signed drift in percentage points. Status never relies on colour alone: the
 * sign is visible, and the over / under / on-target wording is exposed as a
 * tooltip and as screen-reader text.
 */
export function PortfolioDriftBadge({
  driftPp,
  showStatus = false,
}: PortfolioDriftBadgeProps) {
  const bandPp = React.useContext(DriftBandContext);
  const state = driftState(driftPp, bandPp);
  const status = driftStateLabels[state];
  return (
    <Badge
      variant={driftVariants[state]}
      className="ds-drift-badge"
      title={status}
    >
      {formatDeviation(driftPp)}
      {showStatus ? (
        <span aria-hidden> · {status.replace(" target", "")}</span>
      ) : null}
      <span className="ds-sr-only"> {status}</span>
    </Badge>
  );
}
