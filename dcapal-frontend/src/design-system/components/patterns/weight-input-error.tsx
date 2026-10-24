import * as React from "react";

/**
 * Accessibility props for a weight field: marks the input invalid and points
 * it at the visible message rendered by {@link WeightInputError}.
 */
export function weightInputA11y(
  error: string | null,
  messageId: string
): { "aria-invalid"?: true; "aria-describedby"?: string } {
  return error ? { "aria-invalid": true, "aria-describedby": messageId } : {};
}

/** The visible inline message for a weight field that holds unusable text. */
export function WeightInputError({
  id,
  error,
}: {
  id: string;
  error: string | null;
}) {
  if (!error) return null;
  return (
    <span id={id} className="ds-weight-error">
      {error}
    </span>
  );
}
