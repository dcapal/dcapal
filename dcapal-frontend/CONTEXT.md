# DcaPal Frontend

This context helps a self-directed investor define portfolios, review their value over time, and decide how to use new savings or rebalance towards target weights. It presents market data and allocation suggestions; it does not execute trades.

## Portfolio language

**Portfolio**:
A named collection of portfolio assets, a quote currency, target weights, and fee policies. It may include current holdings, but strategic allocation guidance and other planning decisions are separate concepts rather than fields inside the Portfolio.
_Avoid_: Account, order, strategic allocation, when referring to the asset collection

**Portfolio clone**:
A new independent Portfolio copied from an existing Portfolio for experimentation. Changes to the clone do not change the original Portfolio; historical performance is calculated separately from asset time series for whichever Portfolio is selected.
_Avoid_: Portfolio version, scenario, when referring to an experimental copy

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
A broad grouping assigned to a portfolio asset, such as equities, bonds, crypto, commodities, or currency. It is used to group and summarize portfolio assets and may supply default rules for allocation constraints.
_Avoid_: Asset type, tactical sleeve, when referring to the grouping

**Defensive asset**:
An asset class DcaPal treats as the lower-risk bucket for strategic guidance: bonds and cash. This is a planning classification, not a claim that these assets have zero investment risk.
_Avoid_: Risk-free asset, when describing actual investment risk

**Risk-on asset**:
An asset class that is not classified as defensive for strategic guidance, including equities, crypto, commodities, and other user-defined classes.
_Avoid_: Growth asset, when referring to the strategic risk grouping

**Guidance bucket**:
A planning group used by The Bull v1 to express its two outputs: the risk-on bucket contains Equities, Crypto, Commodities, and Other; the defensive bucket contains Bonds and Cash. Guidance sets the two bucket totals without adding a hierarchy below Asset Class.
_Avoid_: Asset Class, tactical sleeve, risk-free bucket

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
The desired percentage used to define an allocation. DcaPal qualifies it as an Asset-class target weight, Absolute target weight, or Relative target weight when the level is important.
_Avoid_: Current weight, allocation amount, when referring to a target

**Asset-class target weight**:
The desired percentage of the whole Portfolio assigned to an Asset Class in Strategic allocation mode. Asset-class target weights across the active Strategic Allocation total 100%.
_Avoid_: Relative target weight, asset weight

**Absolute target weight**:
The desired percentage of the whole Portfolio assigned to a Portfolio Asset in Simple allocation mode. In Strategic allocation mode, it is derived from the Asset-class target weight multiplied by the asset's Relative target weight within that class.
_Avoid_: Relative target weight, current weight

**Relative target weight**:
The desired percentage of an Asset Class assigned to a Portfolio Asset in Strategic allocation mode. Relative target weights for the assets in one class total 100%.
_Avoid_: Absolute target weight, current weight

**Simple allocation mode**:
The mode in which an investor defines Absolute target weights directly for each Portfolio Asset. Asset-class totals are derived from those asset weights.
_Avoid_: Strategic allocation mode, flat mode

**Strategic allocation mode**:
The advanced mode in which an investor defines Asset-class target weights and Relative target weights within each class. DcaPal derives each asset's Absolute target weight from those two levels.
_Avoid_: Tactical sleeve mode, nested allocation

**Strategic allocation**:
A separate planning definition of Asset-class target weights that can be compared with any Portfolio. Each Portfolio has at most one active Strategic Allocation; users explore alternatives by cloning the Portfolio rather than attaching multiple Strategic Allocations.
_Avoid_: Portfolio, when referring to the asset-class plan

**Strategic allocation guidance**:
A separate area where an investor can explore Allocation Frameworks and create or update the Strategic Allocation for a Portfolio. It works in both Simple and Strategic allocation modes by aggregating current asset weights into Asset Class weights; in Simple mode it is comparison-only until the investor explicitly switches to Strategic mode. Preview is read-only and applying guidance is an explicit user action.
_Avoid_: Portfolio editor, financial advice

**Guidance age**:
The investor's completed age in whole years, accepted from 0 through 120 inclusive, used as an input to Strategic allocation guidance. It is not a property of the Portfolio or Strategic Allocation.
_Avoid_: Date of birth, fractional age, automatic age calculation

**Guidance rate type**:
The official live rate selected for an Allocation Framework calculation. In the MVP, the choices are Fed EFFR and ECB €STR; the application uses the latest available observation from the selected source and does not silently switch sources. Live and manual rate values are signed percentage points, with source precision retained for calculation.
_Avoid_: Policy rate, bond yield, fallback rate

**Manual guidance rate**:
A temporary signed rate value, entered in percentage points with up to three decimal places, for one guidance preview or application when the selected live source cannot provide a usable observation. It must be finite and is not saved as Portfolio or Strategic Allocation data; an applied guidance record still identifies the selected official rate type and age, but not the manual value.
_Avoid_: Stored rate, fallback source, policy rate

**Allocation framework**:
A named formula, rule, or literature-based approach that suggests strategic asset-class weights, such as The Bull rule. It is educational reference material; the user decides whether to apply its output.
_Avoid_: Financial advice, automatic recommendation

**Framework version**:
An immutable DcaPal-owned identifier for the exact formula and rate policy used by an Allocation Framework. The MVP uses `the-bull-v1`; any change to its meaning requires a new version.
_Avoid_: Source article version, last-updated date, live rate version

**Drift band**:
An optional Portfolio-wide range around each target weight used to decide whether to surface drift or recommend rebalancing. It is expressed in percentage points; a missing value disables drift diagnostics. Asset- and Asset Class-specific overrides are not part of this MVP.
_Avoid_: Confidence band, when referring to a rebalancing threshold

**Model performance history**:
A normalized time series calculated from target weights and historical asset prices, rebased to a common starting value. It shows how the model Portfolio moved over time and is not a money-weighted return.
_Avoid_: Transaction history, money-weighted return

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
A buy-only recommendation for distributing the investment budget among Portfolio Assets to move projected weights towards target weights. It may include unallocated cash and is not an executed trade.
_Avoid_: Order, transaction, full rebalancing

**Rebalancing**:
Changing existing holdings to restore target weights. It may sell eligible overweight Portfolio Assets and use their proceeds, together with the investment budget, to buy underweight assets. A buy-only allocation can improve balance without being a full rebalancing.
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

**Effective price**:
The price currently used to value a Portfolio asset. Its provenance is explicit: manual price, current provider price, or last-known provider price.
_Avoid_: Current price, when the source or freshness matters

**Historical price series**:
An ordered set of daily provider price observations for an Asset, used to calculate Model performance history. It begins at the first real observation available and is not a transaction history.
_Avoid_: Price chart, transaction history, when referring to the source data

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
