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
import { formatFeeImpact, formatQuantity } from "./invest-format";
import type { ProposedTransaction } from "./invest-types";

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
  className?: string;
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
