import * as React from "react";
import { cn } from "../../lib/cn";
import { Badge } from "./feedback";

export interface AssetTickerChipProps extends Omit<
  React.ComponentPropsWithoutRef<typeof Badge>,
  "children" | "variant"
> {
  /** The exchange ticker displayed as a neutral asset identity chip. */
  ticker: string;
}

/** A shared neutral ticker treatment for asset identities and search results. */
export function AssetTickerChip({
  ticker,
  className,
  ...props
}: AssetTickerChipProps) {
  return (
    <Badge
      variant="neutral"
      className={cn("ds-asset-ticker-chip", className)}
      {...props}
    >
      {ticker}
    </Badge>
  );
}
