import * as React from "react";
import { Field, FieldDescription, FieldError, FieldLabel } from "../ui";

export interface FormFieldProps extends React.ComponentPropsWithoutRef<
  typeof Field
> {
  label: string;
  description?: string;
  error?: string;
  children: React.ReactNode;
}
export function FormField({
  label,
  description,
  error,
  children,
  ...props
}: FormFieldProps) {
  return (
    <Field invalid={Boolean(error)} {...props}>
      <FieldLabel>{label}</FieldLabel>
      {children}
      {description ? <FieldDescription>{description}</FieldDescription> : null}
      {error ? <FieldError match>{error}</FieldError> : null}
    </Field>
  );
}
