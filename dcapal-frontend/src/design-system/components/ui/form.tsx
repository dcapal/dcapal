import * as React from "react";
import { Field as BaseField } from "@base-ui/react/field";
import { cn } from "../../lib/cn";

export interface FieldGroupProps extends React.HTMLAttributes<HTMLDivElement> {}

/** Groups related fields with a consistent vertical rhythm. */
export function FieldGroup({ className, ...props }: FieldGroupProps) {
  return <div className={cn("ds-field-group", className)} {...props} />;
}

export interface FieldProps extends React.ComponentPropsWithoutRef<
  typeof BaseField.Root
> {
  orientation?: "stacked" | "inline";
}

/** Base UI field root with semantic validation state styling. */
export const Field = React.forwardRef<HTMLDivElement, FieldProps>(
  function Field({ className, orientation = "stacked", ...props }, ref) {
    return (
      <BaseField.Root
        ref={ref}
        className={cn(
          "ds-field",
          orientation === "inline" && "ds-field--inline",
          className
        )}
        {...props}
      />
    );
  }
);

export const FieldLabel = React.forwardRef<
  HTMLLabelElement,
  React.ComponentPropsWithoutRef<typeof BaseField.Label>
>(function FieldLabel({ className, ...props }, ref) {
  return (
    <BaseField.Label
      ref={ref}
      className={cn("ds-label", className)}
      {...props}
    />
  );
});

export const FieldDescription = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseField.Description>
>(function FieldDescription({ className, ...props }, ref) {
  return (
    <BaseField.Description
      ref={ref}
      className={cn("ds-description", className)}
      {...props}
    />
  );
});

export const FieldError = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseField.Error>
>(function FieldError({ className, ...props }, ref) {
  return (
    <BaseField.Error
      ref={ref}
      className={cn("ds-error", className)}
      {...props}
    />
  );
});
