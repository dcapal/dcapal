import React, { useState } from "react";
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

const HelpIcon = () => (
  <span className="inline-flex items-center justify-center w-4 h-4 rounded-full bg-neutral-200 text-neutral-600 text-[10px] cursor-help">
    ?
  </span>
);

export const ResponsiveHelpIcon = ({ title, tooltip, isMobile }) => {
  const [open, setOpen] = useState(false);

  if (isMobile) {
    return (
      <Drawer open={open} onOpenChange={setOpen}>
        <DrawerTrigger asChild>
          <button type="button">
            <HelpIcon />
          </button>
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
      <Tooltip>
        <TooltipTrigger asChild>
          <span>
            <HelpIcon />
          </span>
        </TooltipTrigger>
        <TooltipContent className="max-w-[16rem]">
          <p>{tooltip}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
};
