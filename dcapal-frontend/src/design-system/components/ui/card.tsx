import * as React from "react";
import { cn } from "../../lib/cn";

export type CardDensity = "default" | "compact";

export interface CardProps extends React.HTMLAttributes<HTMLElement> {
  /** Controls the shared padding rhythm for the card's named slots. */
  density?: CardDensity;
}

export function Card({ className, density = "default", ...props }: CardProps) {
  return (
    <section
      className={cn("ds-card", `ds-card--${density}`, className)}
      {...props}
    />
  );
}
export function CardHeader({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return <header className={cn("ds-card__header", className)} {...props} />;
}
export function CardTitle({
  className,
  ...props
}: React.HTMLAttributes<HTMLHeadingElement>) {
  return <h2 className={cn("ds-card__title", className)} {...props} />;
}
export function CardDescription({
  className,
  ...props
}: React.HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn("ds-card__description", className)} {...props} />;
}
export function CardContent({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("ds-card__content", className)} {...props} />;
}
export function CardFooter({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return <footer className={cn("ds-card__footer", className)} {...props} />;
}
