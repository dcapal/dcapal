import * as React from "react";
import { cn } from "../../lib/cn";

export type BadgeVariant =
  "neutral" | "primary" | "success" | "warning" | "destructive";

export function Badge({
  className,
  variant = "neutral",
  ...props
}: React.HTMLAttributes<HTMLSpanElement> & { variant?: BadgeVariant }) {
  return (
    <span
      className={cn("ds-badge", `ds-badge--${variant}`, className)}
      {...props}
    />
  );
}

export type AlertVariant = "info" | "success" | "warning" | "destructive";

export function Alert({
  className,
  variant = "info",
  role,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & { variant?: AlertVariant }) {
  return (
    <div
      role={role ?? (variant === "destructive" ? "alert" : "status")}
      className={cn("ds-alert", `ds-alert--${variant}`, className)}
      {...props}
    />
  );
}

export function AlertTitle({
  className,
  ...props
}: React.HTMLAttributes<HTMLHeadingElement>) {
  return <h3 className={cn("ds-card__title", className)} {...props} />;
}

export function AlertDescription({
  className,
  ...props
}: React.HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn("ds-card__description", className)} {...props} />;
}

export function Separator({
  className,
  orientation = "horizontal",
  ...props
}: React.HTMLAttributes<HTMLDivElement> & {
  orientation?: "horizontal" | "vertical";
}) {
  return (
    <div
      role="separator"
      aria-orientation={orientation}
      className={cn(
        "ds-separator",
        orientation === "vertical" && "ds-separator--vertical",
        className
      )}
      {...props}
    />
  );
}

export function Skeleton({
  className,
  ...props
}: React.HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      aria-hidden="true"
      className={cn("ds-skeleton", className)}
      {...props}
    />
  );
}

export interface EmptyStateProps extends React.HTMLAttributes<HTMLDivElement> {
  title: string;
  description?: string;
  action?: React.ReactNode;
}

export function EmptyState({
  title,
  description,
  action,
  className,
  ...props
}: EmptyStateProps) {
  return (
    <div className={cn("ds-empty", className)} {...props}>
      <strong>{title}</strong>
      {description ? <p>{description}</p> : null}
      {action}
    </div>
  );
}

export interface AvatarProps extends React.HTMLAttributes<HTMLSpanElement> {
  src?: string;
  alt?: string;
  fallback: string;
  size?: "sm" | "md" | "lg";
}

export function Avatar({
  src,
  alt = "",
  fallback,
  size = "md",
  className,
  ...props
}: AvatarProps) {
  const [failed, setFailed] = React.useState(false);
  return (
    <span
      className={cn("ds-avatar", `ds-avatar--${size}`, className)}
      {...props}
    >
      {src && !failed ? (
        <img src={src} alt={alt} onError={() => setFailed(true)} />
      ) : (
        fallback
      )}
    </span>
  );
}
