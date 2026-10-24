import {
  ArrowRight,
  CircleAlert,
  CircleCheck,
  TrendingDown,
} from "lucide-react";
import { Alert, Badge, Button } from "../ui";
import { formatDrift, type DriftStatus } from "../../fixtures/portfolio";

export interface DriftDiagnosticProps {
  assetName: string;
  driftPp: number;
  status: DriftStatus;
  onReview?: () => void;
}
/** Explains an asset or class drift using both a status icon and plain language. */
export function DriftDiagnostic({
  assetName,
  driftPp,
  status,
  onReview,
}: DriftDiagnosticProps) {
  const variant =
    status === "over" ? "warning" : status === "under" ? "info" : "success";
  const label =
    status === "over"
      ? "above target"
      : status === "under"
        ? "below target"
        : "within target";
  const Icon =
    status === "over"
      ? CircleAlert
      : status === "under"
        ? TrendingDown
        : CircleCheck;
  return (
    <Alert variant={variant} className="ds-drift">
      <span className="ds-drift__icon" aria-hidden>
        <Icon size={18} strokeWidth={2.2} />
      </span>
      <div className="ds-drift__content">
        <span className="ds-drift__label">
          {assetName} is {label}.
        </span>
        <span className="ds-drift__detail">
          Current drift is{" "}
          <Badge
            variant={
              status === "over"
                ? "warning"
                : status === "under"
                  ? "primary"
                  : "success"
            }
          >
            {formatDrift(driftPp)}
          </Badge>
        </span>
      </div>
      {onReview ? (
        <Button variant="link" onClick={onReview}>
          Review details <ArrowRight size={14} aria-hidden />
        </Button>
      ) : null}
    </Alert>
  );
}
