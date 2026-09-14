import * as React from "react";
import { Tabs as BaseTabs } from "@base-ui/react/tabs";
import { Tooltip as BaseTooltip } from "@base-ui/react/tooltip";
import { cn } from "../../lib/cn";

export const Tabs = BaseTabs.Root;

export const TabsList = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseTabs.List>
>(function TabsList({ className, ...props }, ref) {
  return (
    <BaseTabs.List
      ref={ref}
      className={cn("ds-tabs-list", className)}
      {...props}
    />
  );
});

export const TabsTrigger = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof BaseTabs.Tab>
>(function TabsTrigger({ className, ...props }, ref) {
  return (
    <BaseTabs.Tab ref={ref} className={cn("ds-tab", className)} {...props} />
  );
});

export const TabsContent = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseTabs.Panel>
>(function TabsContent({ className, ...props }, ref) {
  return (
    <BaseTabs.Panel
      ref={ref}
      className={cn("ds-tabs-content", className)}
      {...props}
    />
  );
});

export interface TooltipProps {
  label: string;
  children: React.ReactElement;
}

/** Tooltip wrapper that keeps the accessible label in the Base UI tree. */
export function Tooltip({ label, children }: TooltipProps) {
  return (
    <BaseTooltip.Root>
      <BaseTooltip.Trigger render={children} />
      <BaseTooltip.Portal className="dcapal-theme">
        <BaseTooltip.Positioner>
          <BaseTooltip.Popup className="ds-tooltip-popup">
            {label}
          </BaseTooltip.Popup>
        </BaseTooltip.Positioner>
      </BaseTooltip.Portal>
    </BaseTooltip.Root>
  );
}
