import * as React from "react";
import { cn } from "../../lib/cn";

/** One labelled value in a DescriptionList. */
export interface DescriptionListItem {
  /** Stable key for the row. */
  id?: string;
  /** What the value describes. */
  label: React.ReactNode;
  /** The value, already formatted for display. */
  value: React.ReactNode;
}

export interface DescriptionListProps extends Omit<
  React.HTMLAttributes<HTMLDListElement>,
  "children"
> {
  /** The rows to show, in order. */
  items: readonly DescriptionListItem[];
}

/** A summary of key and value rows, such as the rules behind a proposal. */
export function DescriptionList({
  items,
  className,
  ...props
}: DescriptionListProps) {
  return (
    <dl className={cn("ds-description-list", className)} {...props}>
      {items.map((item, index) => (
        <div className="ds-description-list__row" key={item.id ?? `${index}`}>
          <dt>{item.label}</dt>
          <dd>{item.value}</dd>
        </div>
      ))}
    </dl>
  );
}
