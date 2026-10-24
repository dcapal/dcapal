import * as React from "react";
import { ChevronDown } from "lucide-react";
import { formatCurrency, formatPercent } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import {
  Badge,
  Button,
  DescriptionList,
  EmptyState,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  type BadgeVariant,
  type DescriptionListItem,
} from "../ui";
import { collapsesToClassRow, isClassOnly } from "./invest-classes";
import { formatFeeImpact, formatQuantity } from "./invest-format";
import type {
  ProposalClassAsset,
  ProposalClassGroup,
  ProposedTransaction,
} from "./invest-types";

const actionLabel = { buy: "Buy", sell: "Sell", hold: "Hold" } as const;
const actionVariant: Record<ProposedTransaction["action"], BadgeVariant> = {
  buy: "success",
  sell: "destructive",
  hold: "neutral",
};

export interface ProposedTransactionCollectionProps {
  transactions: readonly ProposedTransaction[];
  /** Quote currency of every amount. */
  currency: string;
  /** Ids of the transactions whose details start open. */
  defaultExpandedIds?: readonly string[];
  /**
   * Strategic allocation: transactions grouped by asset class. When given, the
   * collection lists a summary row per class with its assets nested below, and
   * `transactions` is not listed flat.
   */
  groups?: readonly ProposalClassGroup[];
  className?: string;
}

/** Formats money moved with an explicit sign, using a true minus sign. */
function formatNetMoney(value: number, currency: string): string {
  if (value > 0) return `+${formatCurrency(value, currency)}`;
  if (value < 0) return `−${formatCurrency(Math.abs(value), currency)}`;
  return formatCurrency(0, currency);
}

/** Weight change and target of a class, with its transaction counts and what cash does. */
function classSummary(group: ProposalClassGroup): string {
  const parts = [
    `${formatPercent(group.weightBefore)} → ${formatPercent(group.weightAfter)}`,
    `target ${formatPercent(group.targetWeight)}`,
  ];
  if (group.buys > 0) {
    parts.push(`${group.buys} ${group.buys === 1 ? "buy" : "buys"}`);
  }
  if (group.sells > 0) {
    parts.push(`${group.sells} ${group.sells === 1 ? "sell" : "sells"}`);
  }
  if (isClassOnly(group.assetClass)) {
    if (group.netMoney > 0) parts.push("unallocated money stays here");
    if (group.netMoney < 0) parts.push("cash balance funds the buys");
  }
  return parts.join(" · ");
}

/** Weights of an asset inside a class: before, after, target and share of the class. */
function assetSummary({ row, shareOfClass }: ProposalClassAsset): string {
  return `${formatPercent(row.weightBefore)} → ${formatPercent(row.weightAfter)} · target ${formatPercent(row.targetWeight)} · ${formatPercent(shareOfClass)} of class`;
}

/** Lists what is known about a transaction beyond its quantity and amount. */
function detailItems(
  transaction: ProposedTransaction,
  currency: string
): DescriptionListItem[] {
  const commission =
    transaction.action === "sell"
      ? "Not estimated for sales"
      : transaction.commission === undefined
        ? "—"
        : formatCurrency(transaction.commission, currency);
  const firstYear =
    transaction.firstYearFeeImpact === undefined
      ? "—"
      : formatCurrency(transaction.firstYearFeeImpact, currency);

  return [
    { id: "commission", label: "Commission", value: commission },
    ...(transaction.action === "sell"
      ? []
      : [
          {
            id: "first-year",
            label: "First-year fee impact",
            value: firstYear,
          },
        ]),
    {
      id: "weight",
      label: "Weight",
      value: `${formatPercent(transaction.weightBefore)} → ${formatPercent(transaction.weightAfter)}`,
    },
    {
      id: "target",
      label: "Target",
      value: formatPercent(transaction.targetWeight),
    },
  ];
}

/** The details of one transaction, including the "should have" block of a fee-rejected one. */
function TransactionDetails({
  transaction,
  currency,
}: {
  transaction: ProposedTransaction;
  currency: string;
}) {
  const { feeRejected } = transaction;

  return (
    <div className="ds-transaction-details">
      <DescriptionList items={detailItems(transaction, currency)} />
      {feeRejected ? (
        <div className="ds-transaction-details__should-have">
          <strong>Should have: {transaction.ticker}</strong>
          <p>
            Buy {formatQuantity(feeRejected.shouldHaveQuantity)} ×{" "}
            {formatCurrency(transaction.price, currency)} ={" "}
            {formatCurrency(feeRejected.shouldHaveAmount, currency)}. The
            commission is {formatFeeImpact(feeRejected.commissionImpact)} of
            that amount, above your {formatFeeImpact(feeRejected.maxFeeImpact)}{" "}
            maximum fee impact.
          </p>
        </div>
      ) : null}
    </div>
  );
}

interface ToggleProps {
  transaction: ProposedTransaction;
  expanded: boolean;
  controlsId: string;
  onToggle: () => void;
  /** Cards show a text label; table rows show only the chevron. */
  showLabel: boolean;
}

/** The keyboard-operable button that shows or hides a transaction's details. */
function DetailsToggle({
  transaction,
  expanded,
  controlsId,
  onToggle,
  showLabel,
}: ToggleProps) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className="ds-transaction-toggle"
      aria-expanded={expanded}
      aria-controls={controlsId}
      aria-label={`Details for ${transaction.ticker}`}
      onClick={onToggle}
    >
      {showLabel ? "Details" : null}
      <ChevronDown aria-hidden />
    </Button>
  );
}

/** A short caption explaining why a held asset has no trade. */
function SkippedNote({ transaction }: { transaction: ProposedTransaction }) {
  return transaction.feeRejected ? (
    <span className="ds-transaction-skipped">Skipped: fee too high</span>
  ) : null;
}

/**
 * Proposed transactions as a table on wide screens and as cards on small
 * screens, with the same labels and the same expandable details in both.
 * Cash is a funding source and never appears as a row.
 */
export function ProposedTransactionCollection({
  transactions,
  currency,
  defaultExpandedIds = [],
  groups,
  className,
}: ProposedTransactionCollectionProps) {
  const idPrefix = React.useId();
  const [expanded, setExpanded] = React.useState<ReadonlySet<string>>(
    () => new Set(defaultExpandedIds)
  );
  const toggle = (id: string) =>
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  if (groups && groups.length > 0) {
    return (
      <GroupedTransactions
        groups={groups}
        currency={currency}
        idPrefix={idPrefix}
        expanded={expanded}
        onToggle={toggle}
        className={className}
      />
    );
  }

  if (transactions.length === 0) {
    return (
      <EmptyState
        title="No proposed transactions"
        description="No transaction would bring your portfolio closer to its targets."
      />
    );
  }

  return (
    <div className={cn("ds-transactions", className)}>
      <ul className="ds-transactions__cards" aria-label="Proposed transactions">
        {transactions.map((transaction) => {
          const detailsId = `${idPrefix}-${transaction.id}-card`;
          const open = expanded.has(transaction.id);
          return (
            <li className="ds-transaction-card" key={transaction.id}>
              <div className="ds-transaction-card__top">
                <Badge variant={actionVariant[transaction.action]}>
                  {actionLabel[transaction.action]}
                </Badge>
                <strong className="ds-transaction-card__amount">
                  {transaction.action === "hold"
                    ? "—"
                    : formatCurrency(transaction.amount, currency)}
                </strong>
              </div>
              <h3 className="ds-transaction-card__title">
                {transaction.ticker}
              </h3>
              <p className="ds-transaction-card__meta">
                {transaction.action === "hold"
                  ? transaction.name
                  : `${formatQuantity(transaction.quantity)} × ${formatCurrency(transaction.price, currency)} · ${transaction.name}`}
              </p>
              <SkippedNote transaction={transaction} />
              <DetailsToggle
                transaction={transaction}
                expanded={open}
                controlsId={detailsId}
                onToggle={() => toggle(transaction.id)}
                showLabel
              />
              <div id={detailsId} hidden={!open}>
                <TransactionDetails
                  transaction={transaction}
                  currency={currency}
                />
              </div>
            </li>
          );
        })}
      </ul>
      <div className="ds-transactions__table">
        <Table
          aria-label="Proposed transactions"
          className="ds-table--transactions"
        >
          <TableHeader>
            <TableRow>
              <TableHead>Asset</TableHead>
              <TableHead>Action</TableHead>
              <TableHead numeric>Quantity</TableHead>
              <TableHead numeric>Price</TableHead>
              <TableHead numeric>Amount</TableHead>
              <TableHead className="ds-table--transactions__toggle">
                <span className="ds-sr-only">Details</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {transactions.flatMap((transaction) => {
              const detailsId = `${idPrefix}-${transaction.id}-row`;
              const open = expanded.has(transaction.id);
              const isHold = transaction.action === "hold";
              return [
                <TableRow key={transaction.id}>
                  <TableCell>
                    <span className="ds-table__asset-identity">
                      <strong>{transaction.ticker}</strong>
                      <span className="ds-table__asset-ticker">
                        {transaction.name}
                      </span>
                    </span>
                  </TableCell>
                  <TableCell>
                    <Badge variant={actionVariant[transaction.action]}>
                      {actionLabel[transaction.action]}
                    </Badge>
                    <SkippedNote transaction={transaction} />
                  </TableCell>
                  <TableCell numeric>
                    {isHold ? "—" : formatQuantity(transaction.quantity)}
                  </TableCell>
                  <TableCell numeric>
                    {formatCurrency(transaction.price, currency)}
                  </TableCell>
                  <TableCell numeric>
                    <strong>
                      {isHold
                        ? "—"
                        : formatCurrency(transaction.amount, currency)}
                    </strong>
                  </TableCell>
                  <TableCell className="ds-table--transactions__toggle">
                    <DetailsToggle
                      transaction={transaction}
                      expanded={open}
                      controlsId={detailsId}
                      onToggle={() => toggle(transaction.id)}
                      showLabel={false}
                    />
                  </TableCell>
                </TableRow>,
                <TableRow
                  key={`${transaction.id}-details`}
                  id={detailsId}
                  hidden={!open}
                  className="ds-table--transactions__details"
                >
                  <TableCell colSpan={6}>
                    <TransactionDetails
                      transaction={transaction}
                      currency={currency}
                    />
                  </TableCell>
                </TableRow>,
              ];
            })}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}

interface GroupedTransactionsProps {
  groups: readonly ProposalClassGroup[];
  currency: string;
  idPrefix: string;
  expanded: ReadonlySet<string>;
  onToggle: (id: string) => void;
  className?: string;
}

/** The Action badge of a grouped asset: its transaction, or Hold when it has none. */
function assetAction(asset: ProposalClassAsset): ProposedTransaction["action"] {
  return asset.transaction?.action ?? "hold";
}

/**
 * Strategic allocation view of the proposed transactions: one summary row per
 * asset class (net money moved, weight before and after against the class
 * target) with its assets nested below. Assets without a transaction stay as
 * muted Hold rows; cash is a class row only. Table and cards share the labels.
 */
function GroupedTransactions({
  groups,
  currency,
  idPrefix,
  expanded,
  onToggle,
  className,
}: GroupedTransactionsProps) {
  return (
    <div className={cn("ds-transactions ds-transactions--grouped", className)}>
      <ul
        className="ds-transactions__cards"
        aria-label="Proposed transactions by asset class"
      >
        {groups.map((group) => {
          const classOnly = collapsesToClassRow(
            group.assetClass,
            group.assets.length
          );
          return (
            <li
              className="ds-transaction-group-card"
              key={group.assetClass}
              data-asset-class={group.assetClass}
              data-row-kind="class"
            >
              <div className="ds-transaction-group-card__header">
                <div className="ds-transaction-card__top">
                  <h3 className="ds-transaction-card__title">{group.label}</h3>
                  <strong className="ds-transaction-card__amount">
                    {formatNetMoney(group.netMoney, currency)}
                  </strong>
                </div>
                <p className="ds-transaction-card__meta">
                  {classSummary(group)}
                </p>
              </div>
              {classOnly ? null : (
                <ul
                  className="ds-transaction-group-card__assets"
                  aria-label={`${group.label} assets`}
                >
                  {group.assets.map((asset) => {
                    const { row, transaction } = asset;
                    const action = assetAction(asset);
                    const detailsId = `${idPrefix}-${row.id}-card`;
                    const open = expanded.has(row.id);
                    return (
                      <li
                        className={cn(
                          "ds-transaction-card ds-transaction-card--nested",
                          action === "hold" && "ds-transaction-card--hold"
                        )}
                        key={row.id}
                        data-row-kind="asset"
                      >
                        <div className="ds-transaction-card__top">
                          <Badge variant={actionVariant[action]}>
                            {actionLabel[action]}
                          </Badge>
                          <strong className="ds-transaction-card__amount">
                            {transaction && action !== "hold"
                              ? formatCurrency(transaction.amount, currency)
                              : "—"}
                          </strong>
                        </div>
                        <h4 className="ds-transaction-card__subtitle">
                          {row.ticker}
                        </h4>
                        <p className="ds-transaction-card__meta">
                          {transaction && action !== "hold"
                            ? `${formatQuantity(transaction.quantity)} × ${formatCurrency(transaction.price, currency)} · `
                            : ""}
                          {row.name}
                        </p>
                        <p className="ds-transaction-card__meta">
                          {assetSummary(asset)}
                        </p>
                        {transaction ? (
                          <>
                            <SkippedNote transaction={transaction} />
                            <DetailsToggle
                              transaction={transaction}
                              expanded={open}
                              controlsId={detailsId}
                              onToggle={() => onToggle(row.id)}
                              showLabel
                            />
                            <div id={detailsId} hidden={!open}>
                              <TransactionDetails
                                transaction={transaction}
                                currency={currency}
                              />
                            </div>
                          </>
                        ) : null}
                      </li>
                    );
                  })}
                </ul>
              )}
            </li>
          );
        })}
      </ul>
      <div className="ds-transactions__table">
        <Table
          aria-label="Proposed transactions by asset class"
          className="ds-table--transactions"
        >
          <TableHeader>
            <TableRow>
              <TableHead>Asset class / asset</TableHead>
              <TableHead>Action</TableHead>
              <TableHead numeric>Quantity</TableHead>
              <TableHead numeric>Price</TableHead>
              <TableHead numeric>Amount</TableHead>
              <TableHead className="ds-table--transactions__toggle">
                <span className="ds-sr-only">Details</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {groups.flatMap((group) => {
              const classOnly = collapsesToClassRow(
                group.assetClass,
                group.assets.length
              );
              const classRow = (
                <TableRow
                  key={group.assetClass}
                  className="ds-table__row--group"
                  data-asset-class={group.assetClass}
                  data-row-kind="class"
                >
                  <TableCell>
                    <span className="ds-table__asset-identity">
                      <strong>{group.label}</strong>
                      <span className="ds-table__asset-ticker">
                        {classSummary(group)}
                      </span>
                    </span>
                  </TableCell>
                  <TableCell />
                  <TableCell numeric />
                  <TableCell numeric />
                  <TableCell numeric>
                    <strong>{formatNetMoney(group.netMoney, currency)}</strong>
                  </TableCell>
                  <TableCell className="ds-table--transactions__toggle" />
                </TableRow>
              );
              if (classOnly) return [classRow];
              return [
                classRow,
                ...group.assets.flatMap((asset) => {
                  const { row, transaction } = asset;
                  const action = assetAction(asset);
                  const moved = transaction !== null && action !== "hold";
                  const detailsId = `${idPrefix}-${row.id}-row`;
                  const open = expanded.has(row.id);
                  return [
                    <TableRow
                      key={row.id}
                      className={cn(
                        "ds-table__row--nested",
                        action === "hold" && "ds-table__row--hold"
                      )}
                      data-asset-class={group.assetClass}
                      data-row-kind="asset"
                    >
                      <TableCell>
                        <span className="ds-table__asset-identity ds-table__asset-identity--nested">
                          <strong>{row.ticker}</strong>
                          <span className="ds-table__asset-ticker">
                            {row.name}
                          </span>
                          <span className="ds-table__asset-ticker">
                            {assetSummary(asset)}
                          </span>
                        </span>
                      </TableCell>
                      <TableCell>
                        <Badge variant={actionVariant[action]}>
                          {actionLabel[action]}
                        </Badge>
                        {transaction ? (
                          <SkippedNote transaction={transaction} />
                        ) : null}
                      </TableCell>
                      <TableCell numeric>
                        {moved ? formatQuantity(transaction.quantity) : "—"}
                      </TableCell>
                      <TableCell numeric>
                        {transaction
                          ? formatCurrency(transaction.price, currency)
                          : "—"}
                      </TableCell>
                      <TableCell numeric>
                        <strong>
                          {moved
                            ? formatCurrency(transaction.amount, currency)
                            : "—"}
                        </strong>
                      </TableCell>
                      <TableCell className="ds-table--transactions__toggle">
                        {transaction ? (
                          <DetailsToggle
                            transaction={transaction}
                            expanded={open}
                            controlsId={detailsId}
                            onToggle={() => onToggle(row.id)}
                            showLabel={false}
                          />
                        ) : null}
                      </TableCell>
                    </TableRow>,
                    ...(transaction
                      ? [
                          <TableRow
                            key={`${row.id}-details`}
                            id={detailsId}
                            hidden={!open}
                            className="ds-table--transactions__details"
                          >
                            <TableCell colSpan={6}>
                              <TransactionDetails
                                transaction={transaction}
                                currency={currency}
                              />
                            </TableCell>
                          </TableRow>,
                        ]
                      : []),
                  ];
                }),
              ];
            })}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
