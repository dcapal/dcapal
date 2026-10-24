import * as React from "react";
import {
  Button as BaseButton,
  type ButtonProps as BaseButtonProps,
} from "@base-ui/react/button";
import { cn } from "../../lib/cn";

export type ButtonVariant =
  "primary" | "secondary" | "outline" | "ghost" | "destructive" | "link";
export type ButtonSize = "sm" | "md" | "lg" | "icon";

export interface ButtonProps extends React.ComponentPropsWithoutRef<"button"> {
  /** The semantic action treatment. */
  variant?: ButtonVariant;
  /** The semantic control size. */
  size?: ButtonSize;
  /** Keeps a toggle-like action visibly and semantically pressed. */
  pressed?: boolean;
  /** Base UI composition hook; avoids a second asChild API. */
  render?: BaseButtonProps["render"];
}

/** The primary action primitive for new DcaPal surfaces. */
export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  function Button(
    {
      className,
      variant = "primary",
      size = "md",
      pressed,
      render,
      "aria-pressed": ariaPressed,
      ...props
    },
    ref
  ) {
    return (
      <BaseButton
        ref={ref}
        render={render}
        aria-pressed={pressed === undefined ? ariaPressed : pressed}
        data-pressed={
          pressed === undefined
            ? ariaPressed === undefined
              ? undefined
              : String(ariaPressed)
            : String(pressed)
        }
        className={cn(
          "ds-button",
          `ds-button--${variant}`,
          `ds-button--${size}`,
          className
        )}
        {...props}
      />
    );
  }
);

export interface IconButtonProps extends Omit<ButtonProps, "size"> {
  label: string;
  /** Small icon actions suit dense tables; medium is the default shell action. */
  size?: "sm" | "md";
}

/** A compact, labelled action for icon-only controls. */
export const IconButton = React.forwardRef<HTMLButtonElement, IconButtonProps>(
  function IconButton({ label, size = "md", className, ...props }, ref) {
    return (
      <Button
        ref={ref}
        aria-label={label}
        size="icon"
        className={cn("ds-icon-button", `ds-icon-button--${size}`, className)}
        {...props}
      />
    );
  }
);
