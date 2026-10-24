# DcaPal Backend

DcaPal Backend provides market data, temporary portfolio transfer, and remote storage for authenticated users' saved portfolios. It does not execute trades or calculate allocation recommendations.

## Remote storage language

**Application data store**:
The durable PostgreSQL-backed store for DcaPal users, saved portfolios, and
portfolio assets. It is separate from the service that authenticates users.
_Avoid_: Authentication database; Supabase database

**Authentication service**:
The separate Supabase service that issues and validates user sessions. Its
internal data store holds authentication-service data, not DcaPal saved
portfolios, and does not receive DcaPal schema migrations.
_Avoid_: Application data store; application database

## Market data language

**Market asset**:
A crypto or fiat asset known to the market-data catalog and usable as the base or quote side of a market.
_Avoid_: Asset, when the distinction from a portfolio asset matters; portfolio asset

**Portfolio asset**:
The Portfolio Asset relationship between a saved Portfolio and shared asset metadata. It owns Portfolio-specific data such as quantity held, manual price, average buy price, fee policy, and an optional Asset Class override. Its target rule is stored in the Portfolio Asset allocation row. It is Portfolio data, not an entry in the market-data catalog, and it may represent an equity even when no market asset exists for it.
_Avoid_: Market asset; holding, when referring to the complete Portfolio entry

**Shared asset metadata**:
The canonical provider-bound record in `assets_data`, identified by numeric provider and upper-case symbol. It owns immutable name and currency metadata plus the shared default Asset Class. One record may be linked to many Portfolios.
_Avoid_: Portfolio Asset relationship; current price; display-only asset

**Portfolio Asset relationship**:
The `portfolio_asset` row linking one saved Portfolio to one shared asset metadata record. Its uniqueness is `(portfolio_id, assets_data_id)`, and it never merges holding or allocation values with another Portfolio.
_Avoid_: shared asset metadata; global holding

**Asset Class override**:
An optional class stored on a Portfolio Asset relationship. Null means that the relationship inherits the shared asset metadata default; a non-null value applies only to that Portfolio.
_Avoid_: shared default Asset Class; provider asset type

**Market**:
A tradable pair of two market assets: a base asset and a quote asset. A market may have a current market price.
_Avoid_: Portfolio asset; asset, when referring to the pair

**Market price**:
A time-stamped observation of the value of one unit of a market's base asset expressed in its quote asset.
_Avoid_: Conversion rate, when the price belongs to a specific market

**Conversion rate**:
The price of one known asset in another known asset, obtained from a direct market or from intermediary markets. When the two assets are the same, the conversion rate is one.
_Avoid_: Market price, when the value is not tied to one specific market; trade price

**Price series**:
An ordered set of daily provider observations for a provider-bound market, with a source timestamp and value for each point.
_Avoid_: Historical chart, when referring to stored market data

**Market data provider**:
An external service that discovers market assets and markets or supplies market prices to DcaPal Backend.
_Avoid_: Portfolio asset provider; broker

## Portfolio storage language

**User**:
An authenticated investor who owns saved portfolios in remote storage.
_Avoid_: Account; customer

**Saved portfolio**:
The server-owned copy of a user's portfolio used for portfolio synchronization. It contains the portfolio definition and its portfolio assets; it is not an order or an executed trade.
_Avoid_: Imported portfolio; account; order

**Portfolio ownership**:
A saved portfolio belongs to one user, and only that user can change or delete it through portfolio synchronization.
_Avoid_: Shared portfolio; global portfolio

**Imported portfolio**:
A temporary portfolio transfer resource created from a valid portfolio definition so a client can fetch it later. It is not owned by an authenticated user and is not a saved portfolio.
_Avoid_: Saved portfolio; portfolio synchronization

**Portfolio synchronization**:
A bidirectional exchange of a user's saved portfolios between the client and backend. A later client version updates the saved portfolio, while a later server version is returned to the client; synchronization also reports saved portfolio deletions and does not execute trades.
_Avoid_: Trade synchronization; portfolio import

**Asset reference**:
A provider (`yf` or `kraken`) and an upper-case symbol identifying an asset. The v2 synchronization contract carries assets only as references; the backend resolves their shared metadata from the shared asset directory or the provider, and a Portfolio Asset is identified by its Portfolio and asset reference (see ADR 0005).
_Avoid_: Portfolio Asset ID in the v2 contract; client-supplied asset metadata

**Allocation mode**:
The Portfolio-wide choice between Simple targeting, which uses absolute Portfolio Asset targets, and Strategic targeting, which uses Asset Class targets and relative Portfolio Asset targets.
_Avoid_: Allocation kind; per-asset mode

**Portfolio Asset allocation**:
The target rule owned by one Portfolio Asset relationship. In Simple mode its weight is an absolute Portfolio target; in Strategic mode its weight is relative to the Portfolio Asset's effective Asset Class.
_Avoid_: Holding target, when the target rule is meant

**Strategic Asset Class target**:
The absolute target weight assigned to one Asset Class within a Portfolio in Strategic allocation mode.
_Avoid_: Class weight, when the target is persisted data

**Strategic Portfolio Asset target**:
The relative target weight assigned to one Portfolio Asset within its effective Asset Class in Strategic allocation mode.
_Avoid_: Absolute target, when referring to the Strategic source value

**Portfolio deletion**:
Removal of a saved portfolio from a user's set of portfolios, represented to synchronization clients by a deletion marker.
_Avoid_: Imported portfolio expiry; archive

**Portfolio asset provider**:
The provider identity carried by shared asset metadata. It is different from the market data provider role used by the backend to discover markets and refresh market prices, even when the same external service supplies both.
_Avoid_: Market data provider; broker
