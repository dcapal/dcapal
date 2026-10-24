import * as React from "react";
import { Check, Info } from "lucide-react";
import { formatCurrency, formatPercent } from "../../fixtures/portfolio";
import {
  Alert,
  Badge,
  Button,
  Card,
  CardContent,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  type BadgeVariant,
} from "../ui";
import {
  formatPoints,
  formatQuantity,
  formatQuantityChange,
} from "./invest-format";
import { collapsesToClassRow, groupResultByClass } from "./invest-classes";
import type {
  AllocationMode,
  InvestmentResultChange,
  InvestmentResultModel,
} from "./invest-types";
import { MetricStrip } from "./metric-card";
import { StickyActionBar } from "./sticky-action-bar";

export interface InvestmentResultProps {
  result: InvestmentResultModel;
  /** Strategic mode lists the changes by asset class. Defaults to simple. */
  mode?: AllocationMode;
  /** Returns to the portfolio overview. */
  onBackToPortfolio?: () => void;
  /** Starts a new Invest journey. */
  onPlanAnother?: () => void;
}

/** One-line standing of a class after the update, or null when the result has no class figures. */
function classStanding(
  weightNow: number | null,
  targetWeight: number | null
): string | null {
  return weightNow === null || targetWeight === null
    ? null
    : `${formatPercent(weightNow)} now · target ${formatPercent(targetWeight)}`;
}

function changeVariant(delta: number): BadgeVariant {
  if (delta > 0) return "success";
  if (delta < 0) return "destructive";
  return "neutral";
}

/** The signed quantity change of an asset as a badge. */
function ChangeBadge({ change }: { change: InvestmentResultChange }) {
  const delta = change.quantityAfter - change.quantityBefore;
  return (
    <Badge variant={changeVariant(delta)}>{formatQuantityChange(delta)}</Badge>
  );
}

/** "before → after units" for an asset. */
function quantityText(change: InvestmentResultChange): string {
  return `${formatQuantity(change.quantityBefore)} → ${formatQuantity(change.quantityAfter)} units`;
}

/** Table row with the quantity of one asset before, after and the change. */
function ResultChangeRow({
  change,
  nested = false,
  assetClass,
}: {
  change: InvestmentResultChange;
  nested?: boolean;
  assetClass?: InvestmentResultChange["assetClass"];
}) {
  return (
    <TableRow
      className={nested ? "ds-table__row--nested" : undefined}
      data-asset-class={assetClass}
      data-row-kind={nested ? "asset" : undefined}
    >
      <TableCell>
        <span
          className={
            nested
              ? "ds-table__asset-identity ds-table__asset-identity--nested"
              : "ds-table__asset-identity"
          }
        >
          <strong>{change.ticker}</strong>
          <span className="ds-table__asset-ticker">{change.name}</span>
        </span>
      </TableCell>
      <TableCell numeric>{formatQuantity(change.quantityBefore)}</TableCell>
      <TableCell numeric>
        <strong>{formatQuantity(change.quantityAfter)}</strong>
      </TableCell>
      <TableCell numeric>
        <ChangeBadge change={change} />
      </TableCell>
    </TableRow>
  );
}

/** Mobile card with the quantity of one asset before and after. */
function ResultChangeCard({
  change,
  nested = false,
}: {
  change: InvestmentResultChange;
  nested?: boolean;
}) {
  return (
    <li className="ds-result-card" data-row-kind={nested ? "asset" : undefined}>
      <div className="ds-result-card__top">
        <strong>{change.ticker}</strong>
        <ChangeBadge change={change} />
      </div>
      <p className="ds-result-card__meta">
        {formatQuantity(change.quantityBefore)} {"→"}{" "}
        <strong>{formatQuantity(change.quantityAfter)}</strong> units
      </p>
    </li>
  );
}

/**
 * The summary shown after the investor updated their holdings: the quantity of
 * each asset before and after, the drift now, and the cash balance. It reminds
 * the investor that DcaPal does not place trades.
 */
export function InvestmentResult({
  result,
  mode = "simple",
  onBackToPortfolio,
  onPlanAnother,
}: InvestmentResultProps) {
  const headingId = React.useId();
  const strategic = mode === "strategic";
  const groups = groupResultByClass(result);

  return (
    <div className="ds-result ds-invest-flow">
      <Card aria-labelledby={headingId}>
        <CardContent className="ds-result__body">
          <span className="ds-result__icon" aria-hidden>
            <Check strokeWidth={2.5} />
          </span>
          <h2 className="ds-result__title" id={headingId}>
            Portfolio updated
          </h2>
          <p className="ds-result__lead">
            We saved the new quantities and cleared your budget.
          </p>
          {strategic ? (
            <ul
              className="ds-result__cards ds-result__cards--classes"
              aria-label="Updated quantities by asset class"
            >
              {groups.map((group) => {
                const standing = classStanding(
                  group.weightNow,
                  group.targetWeight
                );
                const classOnly = collapsesToClassRow(
                  group.assetClass,
                  group.changes.length
                );
                return (
                  <li
                    className="ds-result-group-card"
                    key={group.assetClass}
                    data-asset-class={group.assetClass}
                    data-row-kind="class"
                  >
                    <div className="ds-result-group-card__header">
                      <strong>{group.label}</strong>
                      {classOnly ? (
                        <ChangeBadge change={group.changes[0]} />
                      ) : null}
                      {standing ? <span>{standing}</span> : null}
                      {classOnly ? (
                        <span>{quantityText(group.changes[0])}</span>
                      ) : null}
                    </div>
                    {classOnly ? null : (
                      <ul
                        className="ds-result-group-card__assets"
                        aria-label={`${group.label} assets`}
                      >
                        {group.changes.map((change) => (
                          <ResultChangeCard
                            key={change.id}
                            change={change}
                            nested
                          />
                        ))}
                      </ul>
                    )}
                  </li>
                );
              })}
            </ul>
          ) : (
            <ul className="ds-result__cards" aria-label="Updated quantities">
              {result.changes.map((change) => (
                <ResultChangeCard key={change.id} change={change} />
              ))}
            </ul>
          )}
          <div className="ds-result__table">
            <Table aria-label="Updated quantities" className="ds-table--result">
              <TableHeader>
                <TableRow>
                  <TableHead>Asset</TableHead>
                  <TableHead numeric>Before</TableHead>
                  <TableHead numeric>After</TableHead>
                  <TableHead numeric>Change</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {strategic
                  ? groups.flatMap((group) => {
                      const classOnly = collapsesToClassRow(
                        group.assetClass,
                        group.changes.length
                      );
                      const standing = classStanding(
                        group.weightNow,
                        group.targetWeight
                      );
                      return [
                        <TableRow
                          key={group.assetClass}
                          className="ds-table__row--group"
                          data-asset-class={group.assetClass}
                          data-row-kind="class"
                        >
                          <TableCell>
                            <span className="ds-table__asset-identity">
                              <strong>{group.label}</strong>
                              {standing ? (
                                <span className="ds-table__asset-ticker">
                                  {standing}
                                </span>
                              ) : null}
                            </span>
                          </TableCell>
                          <TableCell numeric>
                            {classOnly
                              ? formatQuantity(group.changes[0].quantityBefore)
                              : null}
                          </TableCell>
                          <TableCell numeric>
                            {classOnly ? (
                              <strong>
                                {formatQuantity(group.changes[0].quantityAfter)}
                              </strong>
                            ) : null}
                          </TableCell>
                          <TableCell numeric>
                            {classOnly ? (
                              <ChangeBadge change={group.changes[0]} />
                            ) : null}
                          </TableCell>
                        </TableRow>,
                        ...(classOnly
                          ? []
                          : group.changes.map((change) => (
                              <ResultChangeRow
                                key={change.id}
                                change={change}
                                nested
                                assetClass={group.assetClass}
                              />
                            ))),
                      ];
                    })
                  : result.changes.map((change) => (
                      <ResultChangeRow key={change.id} change={change} />
                    ))}
              </TableBody>
            </Table>
          </div>
          <MetricStrip
            label="Portfolio after the update"
            metrics={[
              { label: "Drift now", value: formatPoints(result.driftNow) },
              {
                label: "Cash balance",
                value: formatCurrency(result.cashNow, result.currency),
              },
            ]}
          />
          <Alert variant="info">
            <Info aria-hidden size={18} />
            <p className="ds-card__description">
              DcaPal does not place trades. Make these transactions with your
              broker.
            </p>
          </Alert>
          <StickyActionBar
            label="Result actions"
            secondary={
              <Button variant="link" onClick={onPlanAnother}>
                Plan another investment
              </Button>
            }
            primary={
              <Button onClick={onBackToPortfolio}>Back to portfolio</Button>
            }
          />
        </CardContent>
      </Card>
    </div>
  );
}
