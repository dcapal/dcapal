import { assetClassLabels, type AssetClass } from "../../fixtures/portfolio";
import { AssetTickerChip } from "../ui";

/** The small class-coloured dot that leads an asset-class label. */
export function AssetClassDot() {
  return <span className="ds-asset-class-dot" aria-hidden />;
}

export interface PortfolioAssetIdentityProps {
  name: string;
  ticker: string;
  /** Asset class shown as muted text after the chip when `showClass` is set. */
  assetClass?: AssetClass;
  /** Shows the class label after the ticker chip (ungrouped rows only). */
  showClass?: boolean;
}

/**
 * The asset identity shared by every holdings table: the name beside its
 * ticker chip and, optionally, the muted asset class. Table and card
 * surfaces render holdings with this one block so they look identical.
 */
export function PortfolioAssetIdentity({
  name,
  ticker,
  assetClass,
  showClass = false,
}: PortfolioAssetIdentityProps) {
  return (
    <span className="ds-asset-identity">
      <strong className="ds-asset-identity__name" title={name}>
        {name}
      </strong>
      <AssetTickerChip ticker={ticker} />
      {showClass && assetClass ? (
        <span className="ds-asset-identity__class">
          {assetClassLabels[assetClass]}
        </span>
      ) : null}
    </span>
  );
}

export interface PortfolioClassIdentityProps {
  label: string;
  /** Muted text after the label, such as the asset count or the class standing. */
  detail?: React.ReactNode;
  /** Leading control, such as the table's collapse button. */
  leading?: React.ReactNode;
  /** Shows the class-coloured dot. Defaults to true; off for the portfolio total. */
  dot?: boolean;
}

/**
 * The asset-class identity of a group row: class dot, label and detail. The
 * table adds its collapse button through `leading`.
 */
export function PortfolioClassIdentity({
  label,
  detail,
  leading,
  dot = true,
}: PortfolioClassIdentityProps) {
  return (
    <span className="ds-class-identity">
      {leading}
      {dot ? <AssetClassDot /> : null}
      <strong>{label}</strong>
      {detail ? (
        <span className="ds-class-identity__detail">{detail}</span>
      ) : null}
    </span>
  );
}
