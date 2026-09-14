import * as React from "react";
import { Checkbox as BaseCheckbox } from "@base-ui/react/checkbox";
import { RadioGroup as BaseRadioGroup } from "@base-ui/react/radio-group";
import { Radio as BaseRadio } from "@base-ui/react/radio";
import { Switch as BaseSwitch } from "@base-ui/react/switch";
import { Check, Circle } from "lucide-react";
import { cn } from "../../lib/cn";

export const Checkbox = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof BaseCheckbox.Root>
>(function Checkbox({ className, children, ...props }, ref) {
  return (
    <BaseCheckbox.Root
      ref={ref}
      className={cn("ds-checkbox", className)}
      {...props}
    >
      {children ?? (
        <BaseCheckbox.Indicator aria-hidden>
          <Check strokeWidth={2.5} />
        </BaseCheckbox.Indicator>
      )}
    </BaseCheckbox.Root>
  );
});

export const RadioGroup = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseRadioGroup>
>(function RadioGroup({ className, ...props }, ref) {
  return (
    <BaseRadioGroup
      ref={ref}
      className={cn("ds-field-group", className)}
      {...props}
    />
  );
});

export const Radio = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof BaseRadio.Root>
>(function Radio({ className, children, ...props }, ref) {
  return (
    <BaseRadio.Root ref={ref} className={cn("ds-radio", className)} {...props}>
      {children ?? (
        <BaseRadio.Indicator aria-hidden>
          <Circle fill="currentColor" strokeWidth={0} />
        </BaseRadio.Indicator>
      )}
    </BaseRadio.Root>
  );
});

export const Switch = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof BaseSwitch.Root>
>(function Switch({ className, ...props }, ref) {
  return (
    <BaseSwitch.Root
      ref={ref}
      className={cn("ds-switch", className)}
      {...props}
    />
  );
});

export interface SegmentedControlOption {
  /** Stable value submitted by the mutually exclusive control. */
  value: string;
  /** Human-readable option label. */
  label: React.ReactNode;
  /** Prevents selecting this option while preserving its visible geometry. */
  disabled?: boolean;
}

export interface SegmentedControlProps extends Omit<
  React.ComponentPropsWithoutRef<typeof BaseRadioGroup>,
  "children" | "value" | "defaultValue" | "onValueChange" | "aria-label"
> {
  /** Accessible name announced for the option group. */
  label: string;
  /** The finite set of mutually exclusive options. */
  options: readonly SegmentedControlOption[];
  /** Controlled selected value. */
  value?: string;
  /** Initial selected value for an uncontrolled group. */
  defaultValue?: string;
  /** Called with the newly selected semantic value. */
  onValueChange?: (value: string) => void;
  /** Compact controls match dense settings and fee forms. */
  size?: "sm" | "md";
}

/** A Base UI radio group presented as a compact, exclusive button row. */
export const SegmentedControl = React.forwardRef<
  HTMLDivElement,
  SegmentedControlProps
>(function SegmentedControl(
  {
    className,
    label,
    options,
    value,
    defaultValue,
    onValueChange,
    size = "md",
    ...props
  },
  ref
) {
  const optionIdPrefix = React.useId();

  return (
    <BaseRadioGroup
      ref={ref}
      aria-label={label}
      className={cn(
        "ds-segmented-control",
        `ds-segmented-control--${size}`,
        className
      )}
      value={value}
      defaultValue={defaultValue}
      onValueChange={(nextValue) => onValueChange?.(String(nextValue))}
      {...props}
    >
      {options.map((option, index) => {
        const optionLabelId = `${optionIdPrefix}-option-${index}`;
        return (
          <label className="ds-segmented-control__option" key={option.value}>
            <BaseRadio.Root
              value={option.value}
              disabled={option.disabled}
              aria-labelledby={optionLabelId}
              className="ds-segmented-control__radio"
            />
            <span id={optionLabelId}>{option.label}</span>
          </label>
        );
      })}
    </BaseRadioGroup>
  );
});

export interface RadioCardGroupOption {
  /** Stable value submitted by the mutually exclusive card group. */
  value: string;
  /** Title shown for the option. */
  title: React.ReactNode;
  /** Supporting explanation shown below the title. */
  description?: React.ReactNode;
  /** Prevents selecting this option while preserving its visible geometry. */
  disabled?: boolean;
}

export interface RadioCardGroupProps extends Omit<
  React.ComponentPropsWithoutRef<typeof BaseRadioGroup>,
  "children" | "value" | "defaultValue" | "onValueChange" | "aria-label"
> {
  /** Accessible and visible name for the card group. */
  label: string;
  /** The finite set of titled and described mutually exclusive cards. */
  options: readonly RadioCardGroupOption[];
  /** Controlled selected value. */
  value?: string;
  /** Initial selected value for an uncontrolled group. */
  defaultValue?: string;
  /** Called with the newly selected semantic value. */
  onValueChange?: (value: string) => void;
}

/** A Base UI radio group rendered as accessible, titled selection cards. */
export const RadioCardGroup = React.forwardRef<
  HTMLDivElement,
  RadioCardGroupProps
>(function RadioCardGroup(
  { className, label, options, value, defaultValue, onValueChange, ...props },
  ref
) {
  const optionIdPrefix = React.useId();

  return (
    <BaseRadioGroup
      ref={ref}
      aria-label={label}
      className={cn("ds-radio-card-group", className)}
      value={value}
      defaultValue={defaultValue}
      onValueChange={(nextValue) => onValueChange?.(String(nextValue))}
      {...props}
    >
      <span className="ds-radio-card-group__label">{label}</span>
      <span className="ds-radio-card-group__options">
        {options.map((option, index) => {
          const optionLabelId = `${optionIdPrefix}-option-${index}`;
          return (
            <label
              className="ds-radio-card-group__option"
              data-disabled={option.disabled ? "" : undefined}
              key={option.value}
            >
              <BaseRadio.Root
                value={option.value}
                disabled={option.disabled}
                aria-labelledby={optionLabelId}
                className="ds-radio-card-group__radio"
              >
                <BaseRadio.Indicator
                  aria-hidden
                  className="ds-radio-card-group__indicator"
                />
              </BaseRadio.Root>
              <span className="ds-radio-card-group__copy">
                <span id={optionLabelId} className="ds-radio-card-group__title">
                  {option.title}
                </span>
                {option.description ? (
                  <span className="ds-radio-card-group__description">
                    {option.description}
                  </span>
                ) : null}
              </span>
            </label>
          );
        })}
      </span>
    </BaseRadioGroup>
  );
});
