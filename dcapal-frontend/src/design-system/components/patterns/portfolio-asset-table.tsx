import * as React from "react";
import {
  ArrowDown,
  ArrowUp,
  ChevronDown,
  MoreVertical,
  Pencil,
} from "lucide-react";
import {
  AssetTickerChip,
  IconButton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../ui";
import {
  assetClassLabels,
  formatCurrency,
  formatPercent,
  type AssetClass,
  type PortfolioAsset,
} from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import { PortfolioDriftBadge } from "./portfolio-asset-drift-badge";
import {
  assetRegion,
  buildPortfolioAssetSummary,
  sortAssetEntries,
  sortAssetGroups,
  type PortfolioAssetClassGroup,
  type PortfolioAssetEntry,
  type PortfolioAssetSortKey,
  type PortfolioAssetSummary,
  type StrategicTargets,
} from "./portfolio-asset-groups";
import {
  usePortfolioAssetViewState,
  type PortfolioAssetMode,
  type PortfolioAssetViewState,
} from "./use-portfolio-asset-view";

export interface PortfolioAssetTableProps {
  assets: readonly PortfolioAsset[];
  /**
   * `simple` lists every asset in one flat table. `strategic` groups assets
   * under their asset class with aggregate rows.
   */
  mode?: PortfolioAssetMode;
  /** Overview keeps the most useful comparison fields in the available workspace width. */
  presentation?: "detailed" | "overview";
  /**
   * Sort and collapse state. Pass the state owned by the collection so the
   * table and the mobile cards agree; omitted, the table keeps its own.
   */
  view?: PortfolioAssetViewState;
  /** Stated Strategic allocation; class targets and shares are shown as entered. */
  targets?: StrategicTargets;
  onEdit?: (asset: PortfolioAsset) => void;
  /** Edits an asset class's target weight; shows an `Edit {Class}` button on class rows. */
  onEditClass?: (assetClass: AssetClass) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}

type ColumnKey =
  | "asset"
  | "class"
  | "quantity"
  | "price"
  | "cost"
  | "value"
  | "current"
  | "target"
  | "drift";

interface Column {
  key: ColumnKey;
  label: string;
  numeric: boolean;
  sortKey?: PortfolioAssetSortKey;
}

const columnDefinitions: Record<ColumnKey, Column> = {
  asset: { key: "asset", label: "Asset", numeric: false, sortKey: "name" },
  class: {
    key: "class",
    label: "Asset class",
    numeric: false,
    sortKey: "class",
  },
  quantity: { key: "quantity", label: "Quantity", numeric: true },
  price: { key: "price", label: "Current price", numeric: true },
  cost: { key: "cost", label: "Average cost basis", numeric: true },
  value: {
    key: "value",
    label: "Current value",
    numeric: true,
    sortKey: "value",
  },
  current: {
    key: "current",
    label: "Current weight",
    numeric: true,
    sortKey: "current",
  },
  target: {
    key: "target",
    label: "Target weight",
    numeric: true,
    sortKey: "target",
  },
  drift: { key: "drift", label: "Drift", numeric: true, sortKey: "drift" },
};

/** Columns shown for a mode and column-density variant. */
function columnsFor(mode: PortfolioAssetMode, overview: boolean): ColumnKey[] {
  if (overview) return ["asset", "value", "current", "target", "drift"];
  // Simple mode has no group rows, so the class gets its own column.
  return [
    "asset",
    ...(mode === "simple" ? (["class"] as const) : []),
    "quantity",
    "price",
    "cost",
    "value",
    "current",
    "target",
    "drift",
  ];
}

function formatQuantity(asset: PortfolioAsset): string {
  return asset.quantity ? asset.quantity.toFixed(2) : "—";
}

/** A value with an optional muted second line aligned to the same edge. */
function Stacked({
  children,
  sub,
}: {
  children: React.ReactNode;
  sub?: React.ReactNode;
}) {
  return (
    <span className="ds-table__stacked">
      <span>{children}</span>
      {sub ? <span className="ds-table__sub">{sub}</span> : null}
    </span>
  );
}

function AssetClassDot() {
  return <span className="ds-asset-class-dot" aria-hidden />;
}

/** Sortable (or plain) column header cell. */
function HeaderCell({
  column,
  view,
}: {
  column: Column;
  view: PortfolioAssetViewState;
}) {
  const { sortKey } = column;
  if (!sortKey) {
    return (
      <TableHead numeric={column.numeric}>
        <span className="ds-table__header-label">{column.label}</span>
      </TableHead>
    );
  }
  const active = view.sort.key === sortKey;
  const ariaSort = active
    ? view.sort.direction === "asc"
      ? "ascending"
      : "descending"
    : "none";
  return (
    <TableHead numeric={column.numeric} aria-sort={ariaSort}>
      <button
        type="button"
        className="ds-table__sort"
        onClick={() => view.onSort(sortKey)}
      >
        <span>{column.label}</span>
        <span className="ds-table__sort-indicator" aria-hidden>
          {active ? (
            view.sort.direction === "asc" ? (
              <ArrowUp size={12} />
            ) : (
              <ArrowDown size={12} />
            )
          ) : null}
        </span>
      </button>
    </TableHead>
  );
}

interface AssetRowProps {
  entry: PortfolioAssetEntry;
  columns: ColumnKey[];
  /** Set inside a strategic group to show shares of the class. */
  grouped: boolean;
  overview: boolean;
  onEdit?: (asset: PortfolioAsset) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}

/** One asset row; the left rail carries the class colour inside a group. */
function AssetRow({
  entry,
  columns,
  grouped,
  overview,
  onEdit,
  onMenu,
}: AssetRowProps) {
  const { asset } = entry;
  const cell = (key: ColumnKey) => {
    switch (key) {
      case "asset":
        return (
          <TableCell
            key={key}
            className={cn(
              "ds-table__asset-cell",
              grouped && "ds-table__asset-cell--grouped"
            )}
          >
            <span className="ds-table__asset-identity">
              <strong className="ds-table__asset-name" title={asset.name}>
                {asset.name}
              </strong>
              <AssetTickerChip ticker={asset.ticker} />
              {!grouped && overview ? (
                <span className="ds-table__sub ds-table__asset-class">
                  {assetClassLabels[asset.assetClass]}
                </span>
              ) : null}
            </span>
          </TableCell>
        );
      case "class":
        return (
          <TableCell key={key}>
            <span className="ds-table__class">
              <AssetClassDot />
              <span>{assetClassLabels[asset.assetClass]}</span>
              <span className="ds-table__sub">{assetRegion(asset)}</span>
            </span>
          </TableCell>
        );
      case "quantity":
        return (
          <TableCell key={key} numeric>
            {formatQuantity(asset)}
          </TableCell>
        );
      case "price":
        return (
          <TableCell key={key} numeric>
            {formatCurrency(asset.price, asset.currency)}
          </TableCell>
        );
      case "cost":
        return (
          <TableCell key={key} numeric>
            {asset.averageCostBasis === undefined
              ? "—"
              : formatCurrency(asset.averageCostBasis, asset.currency)}
          </TableCell>
        );
      case "value":
        return (
          <TableCell key={key} numeric>
            {formatCurrency(asset.value, asset.currency)}
          </TableCell>
        );
      case "current":
        return (
          <TableCell key={key} numeric>
            <Stacked
              sub={
                grouped
                  ? `${formatPercent(entry.currentShareOfClass)} of class`
                  : undefined
              }
            >
              {formatPercent(entry.currentWeight)}
            </Stacked>
          </TableCell>
        );
      case "target":
        return (
          <TableCell key={key} numeric>
            <Stacked
              sub={
                grouped
                  ? `${formatPercent(entry.targetShareOfClass)} of class`
                  : undefined
              }
            >
              {formatPercent(entry.targetWeight)}
            </Stacked>
          </TableCell>
        );
      case "drift":
        return (
          <TableCell key={key} numeric>
            <PortfolioDriftBadge driftPp={entry.driftPp} />
          </TableCell>
        );
    }
  };

  return (
    <TableRow
      className="ds-table__row ds-table__row--asset"
      data-asset-class={grouped ? asset.assetClass : undefined}
    >
      {columns.map(cell)}
      <TableCell className="ds-table__actions">
        {onEdit ? (
          <IconButton
            label={`Edit ${asset.ticker}`}
            variant="ghost"
            size="sm"
            onClick={() => onEdit(asset)}
          >
            <Pencil size={16} aria-hidden />
          </IconButton>
        ) : null}
        {onMenu ? (
          <IconButton
            label={`More actions for ${asset.ticker}`}
            variant="ghost"
            size="sm"
            onClick={() => onMenu(asset)}
          >
            <MoreVertical size={16} aria-hidden />
          </IconButton>
        ) : null}
      </TableCell>
    </TableRow>
  );
}

/** Aggregate row of one asset class with its expand / collapse control. */
function GroupRow({
  group,
  columns,
  expanded,
  onToggle,
  onEditClass,
}: {
  group: PortfolioAssetClassGroup;
  columns: ColumnKey[];
  expanded: boolean;
  onToggle: () => void;
  onEditClass?: (assetClass: AssetClass) => void;
}) {
  const count = group.entries.length;
  const cell = (key: ColumnKey) => {
    switch (key) {
      case "asset":
        return (
          <TableCell key={key}>
            <span className="ds-table__group-identity">
              <IconButton
                label={`${expanded ? "Collapse" : "Expand"} ${group.label}`}
                variant="ghost"
                size="sm"
                aria-expanded={expanded}
                className="ds-table__toggle"
                onClick={onToggle}
              >
                <ChevronDown size={16} aria-hidden />
              </IconButton>
              <AssetClassDot />
              <strong>{group.label}</strong>
              <span className="ds-table__sub">
                {count} {count === 1 ? "asset" : "assets"}
              </span>
            </span>
          </TableCell>
        );
      case "cost":
        return (
          <TableCell key={key} numeric>
            <Stacked
              sub={group.invested === null ? undefined : "total invested"}
            >
              {group.invested === null
                ? "—"
                : formatCurrency(group.invested, group.currency)}
            </Stacked>
          </TableCell>
        );
      case "value":
        return (
          <TableCell key={key} numeric>
            {formatCurrency(group.value, group.currency)}
          </TableCell>
        );
      case "current":
        return (
          <TableCell key={key} numeric>
            {formatPercent(group.currentWeight)}
          </TableCell>
        );
      case "target":
        return (
          <TableCell key={key} numeric>
            {formatPercent(group.targetWeight)}
          </TableCell>
        );
      case "drift":
        return (
          <TableCell key={key} numeric>
            <PortfolioDriftBadge driftPp={group.driftPp} />
          </TableCell>
        );
      default:
        return <TableCell key={key} numeric />;
    }
  };

  return (
    <TableRow
      className="ds-table__row ds-table__row--group"
      data-asset-class={group.assetClass}
    >
      {columns.map(cell)}
      <TableCell className="ds-table__actions">
        {onEditClass ? (
          <IconButton
            label={`Edit ${group.label}`}
            variant="ghost"
            size="sm"
            onClick={() => onEditClass(group.assetClass)}
          >
            <Pencil size={16} aria-hidden />
          </IconButton>
        ) : null}
      </TableCell>
    </TableRow>
  );
}

/** Closing row with the portfolio-wide totals. */
function TotalRow({
  summary,
  columns,
}: {
  summary: PortfolioAssetSummary;
  columns: ColumnKey[];
}) {
  const cell = (key: ColumnKey) => {
    switch (key) {
      case "asset":
        return (
          <TableCell key={key}>
            <span className="ds-table__group-identity ds-table__group-identity--total">
              <strong>Portfolio total</strong>
              <span className="ds-table__sub">
                {summary.entries.length}{" "}
                {summary.entries.length === 1 ? "asset" : "assets"}
              </span>
            </span>
          </TableCell>
        );
      case "cost":
        return (
          <TableCell key={key} numeric>
            <Stacked
              sub={
                summary.totalInvested === null ? undefined : "total invested"
              }
            >
              {summary.totalInvested === null
                ? "—"
                : formatCurrency(summary.totalInvested, summary.currency)}
            </Stacked>
          </TableCell>
        );
      case "value":
        return (
          <TableCell key={key} numeric>
            {formatCurrency(summary.totalValue, summary.currency)}
          </TableCell>
        );
      case "current":
        return (
          <TableCell key={key} numeric>
            {formatPercent(summary.totalCurrentWeight)}
          </TableCell>
        );
      case "target":
        return (
          <TableCell key={key} numeric>
            {formatPercent(summary.totalTargetWeight)}
          </TableCell>
        );
      default:
        return <TableCell key={key} numeric />;
    }
  };
  return (
    <TableRow className="ds-table__row ds-table__row--total">
      {columns.map(cell)}
      <TableCell className="ds-table__actions" />
    </TableRow>
  );
}

/**
 * The desktop portfolio asset table. Simple mode is a flat sortable list;
 * strategic mode groups assets under collapsible asset-class rows and ends
 * with a portfolio total.
 */
export function PortfolioAssetTable({
  assets,
  mode = "simple",
  presentation = "detailed",
  view,
  targets,
  onEdit,
  onEditClass,
  onMenu,
}: PortfolioAssetTableProps) {
  const localView = usePortfolioAssetViewState(undefined, mode);
  const state = view ?? localView;
  const overview = presentation === "overview";
  const columns = columnsFor(mode, overview);
  const summary = React.useMemo(
    () => buildPortfolioAssetSummary(assets, targets),
    [assets, targets]
  );

  return (
    <Table
      aria-label="Portfolio assets"
      className={cn(
        "ds-table--assets",
        overview && "ds-table--assets-overview",
        `ds-table--assets-${mode}`
      )}
    >
      <colgroup>
        {columns.map((key) => (
          <col key={key} className={`ds-table__col--${key}`} />
        ))}
        <col className="ds-table__col--actions" />
      </colgroup>
      <TableHeader>
        <TableRow>
          {columns.map((key) => (
            <HeaderCell
              key={key}
              column={columnDefinitions[key]}
              view={state}
            />
          ))}
          <TableHead className="ds-table__actions">
            <span className="ds-sr-only">Actions</span>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {mode === "simple" ? (
          sortAssetEntries(summary.entries, state.sort).map((entry) => (
            <AssetRow
              key={entry.asset.id}
              entry={entry}
              columns={columns}
              grouped={false}
              overview={overview}
              onEdit={onEdit}
              onMenu={onMenu}
            />
          ))
        ) : (
          <>
            {sortAssetGroups(summary.groups, state.sort).map((group) => {
              const expanded = !state.collapsedClasses.includes(
                group.assetClass
              );
              return (
                <React.Fragment key={group.assetClass}>
                  <GroupRow
                    group={group}
                    columns={columns}
                    expanded={expanded}
                    onToggle={() => state.onToggleClass(group.assetClass)}
                    onEditClass={onEditClass}
                  />
                  {expanded
                    ? group.entries.map((entry) => (
                        <AssetRow
                          key={entry.asset.id}
                          entry={entry}
                          columns={columns}
                          grouped
                          overview={overview}
                          onEdit={onEdit}
                          onMenu={onMenu}
                        />
                      ))
                    : null}
                </React.Fragment>
              );
            })}
            <TotalRow summary={summary} columns={columns} />
          </>
        )}
      </TableBody>
    </Table>
  );
}
