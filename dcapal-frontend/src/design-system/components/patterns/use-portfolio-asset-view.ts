import * as React from "react";
import type { AssetClass } from "../../fixtures/portfolio";
import {
  defaultPortfolioAssetSort,
  nextPortfolioAssetSort,
  normalizePortfolioAssetSort,
  type PortfolioAssetSort,
  type PortfolioAssetSortKey,
} from "./portfolio-asset-groups";

/** How the asset collection arranges assets: a flat list or grouped by class. */
export type PortfolioAssetMode = "simple" | "strategic";

/** Sort and collapse state shared by the asset table and the mobile cards. */
export interface PortfolioAssetViewState {
  sort: PortfolioAssetSort;
  /** Activates a column: flips the direction or switches to a new column. */
  onSort: (key: PortfolioAssetSortKey) => void;
  /** Sets an explicit ordering, for example from the mobile sort control. */
  setSort: (sort: PortfolioAssetSort) => void;
  /** Resets the ordering to the default, for example after a mode change. */
  resetSort: () => void;
  collapsedClasses: readonly AssetClass[];
  onToggleClass: (assetClass: AssetClass) => void;
}

/** Owns the ordering and the collapsed asset classes of one asset view. */
export function usePortfolioAssetViewState(
  defaultCollapsedClasses: readonly AssetClass[] = [],
  mode: PortfolioAssetMode = "simple"
): PortfolioAssetViewState {
  const [rawSort, setSort] = React.useState<PortfolioAssetSort>(
    defaultPortfolioAssetSort
  );
  const [collapsedClasses, setCollapsed] = React.useState<
    readonly AssetClass[]
  >(defaultCollapsedClasses);

  // A class ordering never leaks into strategic mode, which has no such column.
  const sort = normalizePortfolioAssetSort(rawSort, mode);

  return React.useMemo(
    () => ({
      sort,
      onSort: (key) =>
        setSort((current) =>
          nextPortfolioAssetSort(
            normalizePortfolioAssetSort(current, mode),
            key
          )
        ),
      setSort: (next) => setSort(next),
      resetSort: () => setSort(defaultPortfolioAssetSort),
      collapsedClasses,
      onToggleClass: (assetClass) =>
        setCollapsed((current) =>
          current.includes(assetClass)
            ? current.filter((item) => item !== assetClass)
            : [...current, assetClass]
        ),
    }),
    [sort, collapsedClasses, mode]
  );
}
