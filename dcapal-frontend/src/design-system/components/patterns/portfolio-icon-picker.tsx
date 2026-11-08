import * as React from "react";
import { Popover } from "@base-ui/react/popover";
import { Check, MoreHorizontal } from "lucide-react";
import { IconButton } from "../ui";
import {
  PortfolioIcon,
  portfolioIconOptions,
  type PortfolioIconId,
} from "../ui/portfolio-icon";
import { cn } from "../../lib/cn";

export interface PortfolioIconPickerProps {
  value?: PortfolioIconId;
  onValueChange?: (value: PortfolioIconId) => void;
  label?: string;
  className?: string;
}

/** Anchored portfolio identity picker with the complete approved icon set. */
export function PortfolioIconPicker({
  value = "pie-chart",
  onValueChange,
  label = "Customize portfolio icon",
  className,
}: PortfolioIconPickerProps) {
  const [open, setOpen] = React.useState(false);

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger
        render={
          <IconButton
            label={label}
            variant="ghost"
            size="sm"
            className={cn("ds-portfolio-icon-picker__trigger", className)}
          >
            <MoreHorizontal aria-hidden />
          </IconButton>
        }
      />
      <Popover.Portal className="dcapal-theme">
        <Popover.Positioner
          className="ds-portfolio-icon-picker__positioner"
          side="bottom"
          align="end"
          sideOffset={6}
        >
          <Popover.Popup className="ds-portfolio-icon-picker__popup">
            <Popover.Title className="ds-portfolio-icon-picker__title">
              Portfolio icon
            </Popover.Title>
            <div
              className="ds-portfolio-icon-picker__grid"
              role="group"
              aria-label="Portfolio icon choices"
            >
              {portfolioIconOptions.map((option) => (
                <button
                  type="button"
                  key={option.id}
                  className={cn(
                    "ds-portfolio-icon-picker__option",
                    option.id === value &&
                      "ds-portfolio-icon-picker__option--selected"
                  )}
                  aria-label={option.label}
                  aria-pressed={option.id === value}
                  onClick={() => {
                    onValueChange?.(option.id);
                    setOpen(false);
                  }}
                >
                  <PortfolioIcon id={option.id} size={20} />
                  {option.id === value ? (
                    <Check
                      aria-hidden
                      className="ds-portfolio-icon-picker__check"
                      size={12}
                    />
                  ) : null}
                </button>
              ))}
            </div>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  );
}
