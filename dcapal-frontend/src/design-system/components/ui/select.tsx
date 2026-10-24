import * as React from "react";
import { Select as BaseSelect } from "@base-ui/react/select";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "../../lib/cn";

export const Select = BaseSelect.Root;
export const SelectValue = React.forwardRef<
  HTMLSpanElement,
  React.ComponentPropsWithoutRef<typeof BaseSelect.Value>
>(function SelectValue({ className, ...props }, ref) {
  return (
    <BaseSelect.Value
      ref={ref}
      data-slot="select-value"
      className={cn("ds-select-value", className)}
      {...props}
    />
  );
});

export const SelectTrigger = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof BaseSelect.Trigger>
>(function SelectTrigger({ className, children, ...props }, ref) {
  return (
    <BaseSelect.Trigger
      ref={ref}
      className={cn("ds-select-trigger", className)}
      {...props}
    >
      {children}
      <BaseSelect.Icon aria-hidden>
        <ChevronDown strokeWidth={1.8} />
      </BaseSelect.Icon>
    </BaseSelect.Trigger>
  );
});

export const SelectContent = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseSelect.Popup>
>(function SelectContent({ className, children, ...props }, ref) {
  return (
    <BaseSelect.Portal className="dcapal-theme">
      <BaseSelect.Positioner
        className="ds-select-positioner"
        align="start"
        alignItemWithTrigger={false}
        sideOffset={4}
      >
        <BaseSelect.Popup
          ref={ref}
          className={cn("ds-select-popup", className)}
          {...props}
        >
          {children}
        </BaseSelect.Popup>
      </BaseSelect.Positioner>
    </BaseSelect.Portal>
  );
});

export const SelectItem = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseSelect.Item>
>(function SelectItem({ className, children, label, ...props }, ref) {
  const itemLabel =
    label ?? (typeof children === "string" ? children : undefined);
  return (
    <BaseSelect.Item
      ref={ref}
      className={cn("ds-select-item", className)}
      {...props}
      label={itemLabel}
    >
      <BaseSelect.ItemIndicator
        aria-hidden
        className="ds-select-item__indicator"
      >
        <Check strokeWidth={2.25} />
      </BaseSelect.ItemIndicator>
      <BaseSelect.ItemText>{children}</BaseSelect.ItemText>
    </BaseSelect.Item>
  );
});

export const SelectGroup = BaseSelect.Group;
export const SelectLabel = BaseSelect.GroupLabel;
export const SelectSeparator = BaseSelect.Separator;
