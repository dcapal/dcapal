import * as React from "react";
import { cn } from "../../lib/cn";

export interface PageHeaderProps extends React.HTMLAttributes<HTMLDivElement> {
  title: string;
  description?: React.ReactNode;
  actions?: React.ReactNode;
  eyebrow?: React.ReactNode;
  status?: React.ReactNode;
  meta?: React.ReactNode;
}
export function PageHeader({
  title,
  description,
  actions,
  eyebrow,
  status,
  meta,
  className,
  ...props
}: PageHeaderProps) {
  return (
    <header className={cn("ds-page-header", className)} {...props}>
      <div className="ds-page-header__body">
        {eyebrow ? (
          <div className="ds-page-header__eyebrow">{eyebrow}</div>
        ) : null}
        <div className="ds-page-header__title-row">
          <h1 className="ds-page-header__title">{title}</h1>
          {status}
        </div>
        {description ? (
          <p className="ds-page-header__description">{description}</p>
        ) : null}
        {meta ? <div className="ds-page-header__meta">{meta}</div> : null}
      </div>
      {actions ? (
        <div className="ds-page-header__actions">{actions}</div>
      ) : null}
    </header>
  );
}
