import * as React from "react";
import { cn } from "../../lib/cn";

export function Table({
  className,
  ...props
}: React.TableHTMLAttributes<HTMLTableElement>) {
  const accessibleName = props["aria-label"];
  return (
    <div
      className="ds-table-wrap"
      role="region"
      tabIndex={0}
      aria-label={accessibleName ? `${accessibleName} table` : "Data table"}
    >
      <table className={cn("ds-table", className)} {...props} />
    </div>
  );
}
export function TableHeader({
  className,
  ...props
}: React.HTMLAttributes<HTMLTableSectionElement>) {
  return <thead className={className} {...props} />;
}
export function TableBody({
  className,
  ...props
}: React.HTMLAttributes<HTMLTableSectionElement>) {
  return <tbody className={className} {...props} />;
}
export function TableRow({
  className,
  ...props
}: React.HTMLAttributes<HTMLTableRowElement>) {
  return <tr className={className} {...props} />;
}
export function TableHead({
  className,
  numeric = false,
  ...props
}: React.ThHTMLAttributes<HTMLTableCellElement> & { numeric?: boolean }) {
  return (
    <th
      scope="col"
      className={cn(numeric && "ds-table__numeric", className)}
      {...props}
    />
  );
}
export function TableCell({
  className,
  numeric = false,
  ...props
}: React.TdHTMLAttributes<HTMLTableCellElement> & { numeric?: boolean }) {
  return (
    <td className={cn(numeric && "ds-table__numeric", className)} {...props} />
  );
}
