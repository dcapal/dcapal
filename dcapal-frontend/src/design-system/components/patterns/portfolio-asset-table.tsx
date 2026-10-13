import { MoreVertical, Pencil } from "lucide-react";
import {
  IconButton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../ui";
import {
  formatAssetClass,
  formatCurrency,
  formatDrift,
  formatPercent,
  type PortfolioAsset,
} from "../../fixtures/portfolio";

export interface PortfolioAssetTableProps {
  assets: readonly PortfolioAsset[];
  /** Overview keeps the most useful comparison fields in the available workspace width. */
  presentation?: "detailed" | "overview";
  onEdit?: (asset: PortfolioAsset) => void;
  onMenu?: (asset: PortfolioAsset) => void;
}

function DriftCell({ asset }: { asset: PortfolioAsset }) {
  return (
    <TableCell numeric>
      <span
        className={
          asset.driftPp > 1
            ? "ds-status-warning"
            : asset.driftPp < -1
              ? "ds-status-primary"
              : "ds-status-success"
        }
      >
        {formatDrift(asset.driftPp)}
      </span>
    </TableCell>
  );
}

export function PortfolioAssetTable({
  assets,
  presentation = "detailed",
  onEdit,
  onMenu,
}: PortfolioAssetTableProps) {
  const isOverview = presentation === "overview";

  return (
    <Table
      aria-label="Portfolio assets"
      className={isOverview ? "ds-table--portfolio-overview" : undefined}
    >
      <TableHeader>
        <TableRow>
          <TableHead>Asset</TableHead>
          {isOverview ? (
            <>
              <TableHead numeric>Current value</TableHead>
              <TableHead numeric>Target weight</TableHead>
              <TableHead numeric>Current weight</TableHead>
              <TableHead numeric>Drift</TableHead>
            </>
          ) : (
            <>
              <TableHead>Asset class</TableHead>
              <TableHead numeric>Quantity</TableHead>
              <TableHead numeric>Current price</TableHead>
              <TableHead numeric>Average cost basis</TableHead>
              <TableHead numeric>Current value</TableHead>
              <TableHead numeric>Target weight</TableHead>
              <TableHead numeric>Current weight</TableHead>
              <TableHead numeric>Drift</TableHead>
            </>
          )}
          <TableHead className="ds-table__actions">
            <span className="ds-sr-only">Actions</span>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {assets.map((asset) => (
          <TableRow key={asset.id}>
            <TableCell>
              <span className="ds-table__asset-identity">
                <strong className="ds-table__asset-name">{asset.name}</strong>
                <span className="ds-table__asset-ticker">{asset.ticker}</span>
              </span>
            </TableCell>
            {isOverview ? (
              <>
                <TableCell numeric>
                  {formatCurrency(asset.value, asset.currency)}
                </TableCell>
                <TableCell numeric>
                  {formatPercent(asset.targetWeight)}
                </TableCell>
                <TableCell numeric>
                  {formatPercent(asset.currentWeight)}
                </TableCell>
                <DriftCell asset={asset} />
              </>
            ) : (
              <>
                <TableCell>{formatAssetClass(asset)}</TableCell>
                <TableCell numeric>
                  {asset.quantity ? asset.quantity.toFixed(2) : "—"}
                </TableCell>
                <TableCell numeric>
                  {formatCurrency(asset.price, asset.currency)}
                </TableCell>
                <TableCell numeric>
                  {asset.averageCostBasis === undefined
                    ? "—"
                    : formatCurrency(asset.averageCostBasis, asset.currency)}
                </TableCell>
                <TableCell numeric>
                  {formatCurrency(asset.value, asset.currency)}
                </TableCell>
                <TableCell numeric>
                  {formatPercent(asset.targetWeight)}
                </TableCell>
                <TableCell numeric>
                  {formatPercent(asset.currentWeight)}
                </TableCell>
                <DriftCell asset={asset} />
              </>
            )}
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
        ))}
      </TableBody>
    </Table>
  );
}
