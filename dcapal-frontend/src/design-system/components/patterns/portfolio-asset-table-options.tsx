import * as React from "react";
import { ArrowDown, ArrowUp, SlidersHorizontal, X } from "lucide-react";
import {
  Button,
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerTitle,
  DrawerTrigger,
  IconButton,
  SegmentedControl,
} from "../ui";
import type { PortfolioAssetSortKey } from "./portfolio-asset-groups";
import type {
  PortfolioAssetMode,
  PortfolioAssetViewState,
} from "./use-portfolio-asset-view";

export const portfolioAssetModeOptions = [
  { value: "simple", label: "Simple" },
  { value: "strategic", label: "Strategic" },
] as const;

const sortOptions: { value: PortfolioAssetSortKey; label: string }[] = [
  { value: "name", label: "Asset name" },
  { value: "class", label: "Asset class" },
  { value: "value", label: "Current value" },
  { value: "current", label: "Current weight" },
  { value: "target", label: "Target weight" },
  { value: "drift", label: "Drift" },
];

export interface PortfolioAssetTableOptionsProps {
  mode: PortfolioAssetMode;
  /** Receives the raw segmented-control value of the chosen table mode. */
  onModeChange: (value: string) => void;
  /** Hides the Table mode control for surfaces that fix the mode. */
  showModeToggle: boolean;
  view: PortfolioAssetViewState;
  /** Opens the drawer on first render; used by the Canvas-only story. */
  defaultOpen?: boolean;
}

/**
 * The mobile "Table options" drawer: a compact button that opens a bottom
 * sheet holding the Table mode switch and the Sort by control. Changes apply
 * live to the cards behind the sheet; Done (or Escape) closes it and returns
 * focus to the button. Desktop hides the button because the mode switch sits
 * in the header and the table headers sort.
 */
export function PortfolioAssetTableOptions({
  mode,
  onModeChange,
  showModeToggle,
  view,
  defaultOpen = false,
}: PortfolioAssetTableOptionsProps) {
  const sortFieldId = React.useId();
  return (
    <Drawer defaultOpen={defaultOpen}>
      <DrawerTrigger
        render={
          <Button
            variant="outline"
            size="sm"
            className="ds-asset-collection__options-button"
          >
            <SlidersHorizontal size={16} aria-hidden />
            Table options
          </Button>
        }
      />
      <DrawerContent className="ds-table-options-sheet">
        <div className="ds-table-options-sheet__header">
          <div>
            <DrawerTitle>Table options</DrawerTitle>
            <DrawerDescription>
              Choose how the investments are grouped and ordered.
            </DrawerDescription>
          </div>
          <DrawerClose
            render={
              <IconButton label="Close table options" variant="ghost">
                <X aria-hidden />
              </IconButton>
            }
          />
        </div>
        {showModeToggle ? (
          <div className="ds-table-options-sheet__section">
            <SegmentedControl
              label="Table mode"
              size="md"
              block
              options={portfolioAssetModeOptions}
              value={mode}
              onValueChange={onModeChange}
            />
          </div>
        ) : null}
        <div className="ds-table-options-sheet__section ds-table-options-sheet__sort">
          <label
            className="ds-asset-collection__sort-field"
            htmlFor={sortFieldId}
          >
            Sort by
            <select
              id={sortFieldId}
              className="ds-asset-collection__sort-select"
              value={view.sort.key}
              onChange={(event) => {
                const key = event.target.value as PortfolioAssetSortKey;
                // Names start ascending, numeric columns largest first.
                view.setSort({
                  key,
                  direction: key === "name" || key === "class" ? "asc" : "desc",
                });
              }}
            >
              {sortOptions
                .filter(
                  (option) => mode === "simple" || option.value !== "class"
                )
                .map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
            </select>
          </label>
          <IconButton
            label={`Sorted ${
              view.sort.direction === "asc" ? "ascending" : "descending"
            }, reverse sort order`}
            variant="outline"
            onClick={() =>
              view.setSort({
                key: view.sort.key,
                direction: view.sort.direction === "asc" ? "desc" : "asc",
              })
            }
          >
            {view.sort.direction === "asc" ? (
              <ArrowUp size={16} aria-hidden />
            ) : (
              <ArrowDown size={16} aria-hidden />
            )}
          </IconButton>
        </div>
        <DrawerFooter>
          <DrawerClose render={<Button>Done</Button>} />
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}
