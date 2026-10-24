import * as React from "react";
import { Input as BaseInput } from "@base-ui/react/input";
import { cn } from "../../lib/cn";

/** A Base UI input with the DcaPal field treatment. */
export const Input = React.forwardRef<
  HTMLInputElement,
  React.ComponentPropsWithoutRef<typeof BaseInput>
>(function Input({ className, ...props }, ref) {
  return (
    <BaseInput ref={ref} className={cn("ds-input", className)} {...props} />
  );
});

export interface InputWithSuffixProps extends React.ComponentPropsWithoutRef<
  typeof BaseInput
> {
  /** A short semantic unit displayed inside the control, such as EUR or %. */
  suffix: string;
}

/** An input with an integrated currency, percentage, or unit chip. */
export const InputWithSuffix = React.forwardRef<
  HTMLInputElement,
  InputWithSuffixProps
>(function InputWithSuffix({ className, suffix, ...props }, ref) {
  return (
    <span className="ds-input-with-suffix">
      <BaseInput ref={ref} className={cn("ds-input", className)} {...props} />
      <span className="ds-input-with-suffix__chip" aria-hidden="true">
        {suffix}
      </span>
    </span>
  );
});

/** A native textarea with the same semantic token treatment as Input. */
export const Textarea = React.forwardRef<
  HTMLTextAreaElement,
  React.TextareaHTMLAttributes<HTMLTextAreaElement>
>(function Textarea({ className, ...props }, ref) {
  return (
    <textarea ref={ref} className={cn("ds-textarea", className)} {...props} />
  );
});

export const Label = React.forwardRef<
  HTMLLabelElement,
  React.LabelHTMLAttributes<HTMLLabelElement>
>(function Label({ className, ...props }, ref) {
  return <label ref={ref} className={cn("ds-label", className)} {...props} />;
});
