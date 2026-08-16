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
An asset entry in a portfolio, with portfolio-specific data such as quantity held, price, target weight, and fee policy overrides. It is portfolio data, not an entry in the market-data catalog, and it may represent an equity even when no market asset exists for it.
_Avoid_: Market asset; holding, when referring to the complete portfolio entry

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

**Portfolio deletion**:
Removal of a saved portfolio from a user's set of portfolios, represented to synchronization clients by a deletion marker.
_Avoid_: Imported portfolio expiry; archive

**Portfolio asset provider**:
The provider label attached to a portfolio asset to identify the source or pricing convention for that entry. It is different from the market data provider used by the backend to discover markets and refresh market prices.
_Avoid_: Market data provider; broker
