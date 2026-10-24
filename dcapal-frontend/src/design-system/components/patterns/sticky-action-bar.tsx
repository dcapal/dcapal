import * as React from "react";
import { cn } from "../../lib/cn";

export interface StickyActionBarProps {
  /** Accessible name of the group of actions. */
  label: string;
  /** Lower-emphasis action, such as going back. */
  secondary?: React.ReactNode;
  /** The main action of the step. */
  primary: React.ReactNode;
  className?: string;
}

/**
 * The actions of a step. On wide screens they sit in the flow below the
 * content; on small screens they stick to the bottom edge. A page that pins
 * this bar supplies no `pageActions`, so the floating page-actions button
 * never shares the corner; should one appear anyway it rides above the bar
 * through `--dcapal-sticky-action-bar-offset`.
 */
export function StickyActionBar({
  label,
  secondary,
  primary,
  className,
}: StickyActionBarProps) {
  return (
    <div
      role="group"
      aria-label={label}
      className={cn("ds-sticky-action-bar", className)}
    >
      {secondary}
      {primary}
    </div>
  );
}
