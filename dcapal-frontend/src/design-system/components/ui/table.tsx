import * as React from "react";
import { cn } from "../../lib/cn";

/**
 * A scrollable table region. It reports through `data-overflow-end` whether
 * more columns are hidden to the right, so a sticky actions column can show a
 * separating shadow only while other content scrolls underneath it.
 */
export function Table({
  className,
  ...props
}: React.TableHTMLAttributes<HTMLTableElement>) {
  const accessibleName = props["aria-label"];
  const wrapRef = React.useRef<HTMLDivElement>(null);
  const [overflowEnd, setOverflowEnd] = React.useState(false);

  React.useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const update = () =>
      setOverflowEnd(wrap.scrollLeft + wrap.clientWidth < wrap.scrollWidth - 1);
    update();
    wrap.addEventListener("scroll", update, { passive: true });
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    observer?.observe(wrap);
    if (wrap.firstElementChild) observer?.observe(wrap.firstElementChild);
    return () => {
      wrap.removeEventListener("scroll", update);
      observer?.disconnect();
    };
  }, []);

  return (
    <div
      ref={wrapRef}
      className="ds-table-wrap"
      role="region"
      tabIndex={0}
      data-overflow-end={overflowEnd ? "true" : undefined}
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
