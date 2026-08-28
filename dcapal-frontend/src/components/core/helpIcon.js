import React, { useRef, useState } from "react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Drawer,
  DrawerContent,
  DrawerHeader,
  DrawerTitle,
  DrawerDescription,
  DrawerTrigger,
} from "@/components/ui/drawer";

const HelpIcon = React.forwardRef(({ title, ...props }, ref) => (
  <button
    ref={ref}
    type="button"
    aria-label={title}
    className="inline-flex appearance-none items-center justify-center w-4 h-4 p-0 border-0 rounded-full bg-neutral-200 text-neutral-600 text-[10px] cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
    {...props}
  >
    <span aria-hidden="true">?</span>
  </button>
));
HelpIcon.displayName = "HelpIcon";

export const ResponsiveHelpIcon = ({ title, tooltip, isMobile }) => {
  const [open, setOpen] = useState(false);
  const clickShouldOpen = useRef(false);

  if (isMobile) {
    return (
      <Drawer open={open} onOpenChange={setOpen}>
        <DrawerTrigger asChild>
          <HelpIcon title={title} />
        </DrawerTrigger>
        <DrawerContent>
          <DrawerHeader>
            <DrawerTitle>{title}</DrawerTitle>
            <DrawerDescription>{tooltip}</DrawerDescription>
          </DrawerHeader>
        </DrawerContent>
      </Drawer>
    );
  }

  return (
    <TooltipProvider>
      <Tooltip open={open} onOpenChange={setOpen}>
        <TooltipTrigger asChild>
          <HelpIcon
            title={title}
            aria-expanded={open}
            onPointerDown={(event) => {
              clickShouldOpen.current = !open;
              if (open) {
                // Prevent Radix from handling this pointer down so the
                // controlled tooltip closes exactly once on repeated clicks.
                event.preventDefault();
                setOpen(false);
              }
            }}
            onClick={(event) => {
              event.preventDefault();
              const shouldOpen =
                event.detail === 0 ? !open : clickShouldOpen.current;
              clickShouldOpen.current = false;
              setOpen(shouldOpen);
            }}
          />
        </TooltipTrigger>
        <TooltipContent className="max-w-[16rem]">
          <p>{tooltip}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
};
