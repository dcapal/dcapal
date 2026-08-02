# DcaPal Frontend

This context helps a self-directed investor describe a portfolio, record current holdings, and decide how to use new savings to move towards a target allocation. It presents market data and produces allocation recommendations; it does not execute trades.

## Portfolio language

**Portfolio**:
A named collection of portfolio assets, a quote currency, target weights, and fee policies. It combines what the investor currently owns with a strategy for future contributions.
_Avoid_: Account, order

**Asset**:
A tradable instrument or currency that can be added to a portfolio.
_Avoid_: Holding, when referring to the instrument itself

**Portfolio asset**:
An asset included in a portfolio, with portfolio-specific data such as quantity held, market price, target weight, and an optional asset fee override.
_Avoid_: Position

**Holding**:
The part of a portfolio asset that the investor already owns, described by its quantity and current value.
_Avoid_: Target allocation

**Asset class**:
A broad kind of asset: equity, crypto, or currency. It supplies the default rule for whether an allocation uses whole or fractional units.
_Avoid_: Asset type, when referring to the equity/crypto/currency classification

**Symbol**:
The provider-recognised identifier that distinguishes an asset.
_Avoid_: Name, ticker, when the identifier may refer to crypto or currency

**Quantity**:
The number of units currently held or recommended for a portfolio asset.
_Avoid_: Shares, as a generic term for every asset class

**Market price**:
The latest price for one unit of an asset, expressed in the portfolio's quote currency.
_Avoid_: Book price, historical price

**Quote currency**:
The currency used to express a portfolio's market prices, values, fees, and investment budget.
_Avoid_: Portfolio currency, account currency

**Base currency**:
The currency in which an asset's source price is denominated before conversion to the portfolio's quote currency.
_Avoid_: Quote currency

**Current value**:
The value of a holding at the latest market price, expressed in the quote currency.
_Avoid_: Amount, when the meaning is the holding's value rather than new money

**Current weight**:
The percentage of the current portfolio value represented by a portfolio asset's current value.
_Avoid_: Target weight

**Target weight**:
The desired percentage of portfolio value for a portfolio asset. When an allocation is calculated, the target applies to the portfolio after the investment budget is added; a complete target allocation totals 100%.
_Avoid_: Current weight, allocation amount

**Average buy price**:
The average cost paid for one unit of a held asset. If it is not available, the current market price is used when showing portfolio gain.
_Avoid_: Market price

**Portfolio gain**:
The percentage difference between the current value of held assets and their average cost. It is a cost-basis return and does not account for the timing of cash flows.
_Avoid_: MWR, money-weighted return

## Contribution and allocation language

**Investment budget**:
New money or liquidity available for the current allocation. It is separate from the portfolio's current value.
_Avoid_: Portfolio value, balance

**Allocation**:
A recommendation for distributing the investment budget among portfolio assets to move projected weights towards target weights. It may include unallocated cash and is not an executed trade.
_Avoid_: Order, transaction, full rebalancing

**Rebalancing**:
Changing existing holdings, including possible sales, to restore target weights. A buy-only allocation can improve balance without being a full rebalancing.
_Avoid_: Allocation, when sales are part of the intended action

**Buy-only mode**:
An allocation rule that can increase or preserve existing holdings but cannot sell them. The product presents this as “Tax Efficient”; that label describes the reason for the rule, not the rule itself.
_Avoid_: Tax optimisation, automatic tax advice

**Unit rule**:
The constraint that a recommended quantity must use whole units or may use fractional units. It applies to all asset classes, not only to shares.
_Avoid_: Whole shares, fractional shares, when the asset is not an equity

**Budget-use preference**:
The investor's choice about whether the allocation should use as much of the investment budget as the price, unit, and fee constraints allow.
_Avoid_: Guaranteed full allocation

**Unallocated cash**:
The part of the investment budget that an allocation recommendation does not assign to a portfolio asset.
_Avoid_: Unused portfolio value, cash asset, unless the quote currency is actually included as a portfolio asset

## Fee and market-data language

**Transaction fee policy**:
A rule used to estimate the cost of buying a portfolio asset during an allocation. It can be zero, fixed, or variable, and may include a maximum fee impact.
_Avoid_: Fee, when the policy and its constraints are meant

**Asset fee override**:
A transaction fee policy that applies to one portfolio asset instead of the portfolio's default policy.
_Avoid_: Portfolio fee, when the asset-specific policy is meant

**Maximum fee impact**:
The largest fee, as a percentage of an allocation, that the investor accepts when considering a purchase.
_Avoid_: Maximum fee amount

**Price provider**:
A service that supplies asset metadata and market prices.
_Avoid_: Broker, unless the service can execute trades

**Portfolio import**:
The creation of an editable portfolio from saved portfolio data, with its asset prices refreshed in the selected quote currency.
_Avoid_: Portfolio sync

**Portfolio synchronization**:
The exchange of saved portfolios between the local client and remote storage for an authenticated investor. It transfers portfolio definitions and holdings; it does not execute an allocation.
_Avoid_: Trade synchronization
