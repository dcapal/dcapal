import * as React from "react";
import { Check, Info } from "lucide-react";
import { formatCurrency } from "../../fixtures/portfolio";
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
import type { InvestmentResultModel } from "./invest-types";
import { MetricStrip } from "./metric-card";
import { StickyActionBar } from "./sticky-action-bar";

export interface InvestmentResultProps {
  result: InvestmentResultModel;
  /** Returns to the portfolio overview. */
  onBackToPortfolio?: () => void;
  /** Starts a new Invest journey. */
  onPlanAnother?: () => void;
}

function changeVariant(delta: number): BadgeVariant {
  if (delta > 0) return "success";
  if (delta < 0) return "destructive";
  return "neutral";
}

/**
 * The summary shown after the investor updated their holdings: the quantity of
 * each asset before and after, the drift now, and the cash balance. It reminds
 * the investor that DcaPal does not place trades.
 */
export function InvestmentResult({
  result,
  onBackToPortfolio,
  onPlanAnother,
}: InvestmentResultProps) {
  const headingId = React.useId();

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
          <ul className="ds-result__cards" aria-label="Updated quantities">
            {result.changes.map((change) => {
              const delta = change.quantityAfter - change.quantityBefore;
              return (
                <li className="ds-result-card" key={change.id}>
                  <div className="ds-result-card__top">
                    <strong>{change.ticker}</strong>
                    <Badge variant={changeVariant(delta)}>
                      {formatQuantityChange(delta)}
                    </Badge>
                  </div>
                  <p className="ds-result-card__meta">
                    {formatQuantity(change.quantityBefore)} {"→"}{" "}
                    <strong>{formatQuantity(change.quantityAfter)}</strong>{" "}
                    units
                  </p>
                </li>
              );
            })}
          </ul>
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
                {result.changes.map((change) => {
                  const delta = change.quantityAfter - change.quantityBefore;
                  return (
                    <TableRow key={change.id}>
                      <TableCell>
                        <span className="ds-table__asset-identity">
                          <strong>{change.ticker}</strong>
                          <span className="ds-table__asset-ticker">
                            {change.name}
                          </span>
                        </span>
                      </TableCell>
                      <TableCell numeric>
                        {formatQuantity(change.quantityBefore)}
                      </TableCell>
                      <TableCell numeric>
                        <strong>{formatQuantity(change.quantityAfter)}</strong>
                      </TableCell>
                      <TableCell numeric>
                        <Badge variant={changeVariant(delta)}>
                          {formatQuantityChange(delta)}
                        </Badge>
                      </TableCell>
                    </TableRow>
                  );
                })}
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
