import * as React from "react";

/**
 * Wraps design-system content in the `.dcapal-theme` scope so semantic tokens,
 * the Instrument Sans font stack, and the warm-ivory canvas apply.
 *
 * Storybook applies the same scope through its global decorator
 * (`.storybook/preview.tsx`); this component exposes it to the design-sync
 * bundle so previews and generated designs can wrap content identically.
 */
export function DcapalThemeScope({
  children,
  className,
}: {
  children?: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={className ? `dcapal-theme ${className}` : "dcapal-theme"}>
      {children}
    </div>
  );
}
