# Findings: The Bull guidance and Fed/ECB rate sources

Research date: 2026-08-03

## Context

- Ticket: [Research: The Bull framework and Fed/ECB rate sources](https://github.com/dcapal/dcapal/issues/744)
- Wayfinder map: [Portfolio management hub and allocation workflows](https://github.com/dcapal/dcapal/issues/742)
- Product epic: [Portfolio management hub and allocation workflows](https://github.com/dcapal/dcapal/issues/738)

This note records facts from the first-party The Bull site, official Federal Reserve and ECB sources, and the repository. It does not implement product code.

## Executive findings

The first MVP version should define a DcaPal-owned framework version, `the-bull-v1`, with this formula:

```text
equity_pct = 125 - age_years - (5 * risk_free_pct)
defensive_pct = 100 - equity_pct
```

`risk_free_pct` is expressed in percentage points. For example, `3.5` means 3.5%, so the same calculation can be written as `125 - age_years - (500 * rate_decimal)` when a source returns `0.035`.

For the requested live-rate MVP, use these exact selected rate types:

| Selected rate type | Official series | Meaning | Cadence |
| --- | --- | --- | --- |
| `fed_effr` | New York Fed EFFR API | U.S. overnight unsecured federal-funds market rate | Each New York Fed business day; prior business day, about 09:00 ET |
| `ecb_estr` | ECB €STR series `EST.B.EU000A2X2A25.WT` | Euro-area wholesale unsecured overnight borrowing rate | Each TARGET2 business day; 08:00 CET, based on the previous TARGET2 business day |

This is a short-rate variant of the framework. The Bull's later first-party guidance says that a long-term portfolio may instead use a government bond yield with a similar duration, such as a 10-year yield. That is a different input policy and must not be silently mixed into `the-bull-v1`.

When guidance is applied, persist only the agreed fields:

```text
framework_version: "the-bull-v1"
selected_rate_type: "fed_effr" | "ecb_estr"
age_years: integer
```

Fetch the latest available selected rate at apply time, use it only for the calculation, and do not persist the rate value, source observation date, or fetch timestamp. The displayed result may show the live rate and its effective date during the interaction, but those are not part of the applied guidance record.

## Repository context

The repository treats The Bull as the first [Allocation framework](../dcapal-frontend/docs/design/portfolio-management-epic.md), and describes framework guidance as educational material that the investor applies explicitly. It also keeps Strategic allocation and framework suggestions separate from the lean Portfolio ([ADR 001](../dcapal-frontend/docs/adr/001-keep-strategic-allocation-separate-from-portfolio.md)); both Simple and Strategic modes can compare guidance with current Asset Class weights ([ADR 004](../dcapal-frontend/docs/adr/004-support-simple-and-strategic-targeting-modes.md)).

The source review found no The Bull, Fed, or ECB guidance implementation in [`dcapal-frontend/src`](../dcapal-frontend/src), [`dcapal-backend/crates/backend/src`](../dcapal-backend/crates/backend/src), or [`packages/api-client/src`](../packages/api-client/src). The existing repository contract is therefore the design documentation, not an existing code contract.

## The Bull framework

### Owning source and formula

The [first-party The Bull episode on asset allocation](https://www.thebull.it/podcast/asset-allocation-la-regola-di-the-bull/) presents the rule as a starting point for dynamic allocation. It describes the equity allocation as `125 - age - (risk-free rate * 5)`. Its worked example uses age 40 and a risk-free rate of 0.035 to obtain about 67% equities, which confirms the percentage-point form above: `125 - 40 - (0.035 * 500) = 67.5`.

The later [The Bull formula and Merton comparison](https://www.thebull.it/podcast/quanto-devo-investire-in-azioni-una-nuova-formula-e-file-da-scaricare/) repeats the formula and states that it is a simplified answer to the question of how much to put in the risky asset versus the risk-free asset. It explicitly simplifies the portfolio to two assets: equities as the risky asset and some form of bonds as the risk-free asset.

The source does not define a separate mathematical formula for the defensive allocation. For a two-asset portfolio, DcaPal should derive it as the remainder, `100 - equity_pct`.

### Assumptions and limits

The first-party sources support these assumptions:

- **Age reduces equity exposure.** The rule assumes that, on average, an investor wants fewer equities as they approach retirement or the point at which their goals are funded. The source uses whole-number examples such as 39, 40, and 50 years ([asset-allocation episode](https://www.thebull.it/podcast/asset-allocation-la-regola-di-the-bull/), [formula comparison](https://www.thebull.it/podcast/quanto-devo-investire-in-azioni-una-nuova-formula-e-file-da-scaricare/)).
- **A higher risk-free rate reduces equity exposure.** The source's rationale is that higher safe returns make bonds more attractive and reduce the relative attraction of equities. When rates are low, the defensive allocation may provide less return and may be more exposed to rate rises ([asset-allocation episode](https://www.thebull.it/podcast/asset-allocation-la-regola-di-the-bull/)).
- **The input should represent the relevant risk-free opportunity.** The earlier explanation uses government bond yields, including a weighted Treasury/Bund example, and says the exact number is less important than the order of magnitude. The later explanation says that using a 10-year government yield can make sense when the portfolio horizon is long ([asset-allocation episode](https://www.thebull.it/podcast/asset-allocation-la-regola-di-the-bull/), [formula comparison](https://www.thebull.it/podcast/quanto-devo-investire-in-azioni-una-nuova-formula-e-file-da-scaricare/)).
- **The rule is a guide, not a complete risk model.** The source says it does not directly include the investor's subjective risk profile or the expected return of each market, and it calls the rule a rough guide. The investor should still consider risk tolerance and goal horizon ([asset-allocation episode](https://www.thebull.it/podcast/asset-allocation-la-regola-di-the-bull/)).
- **The original core model is two assets.** Gold, commodities, or other alternatives are optional additions. The source's guidance is to reduce the equity and bond percentages proportionally when adding a third asset; that is not part of the core two-asset formula ([The Bull checklist](https://www.thebull.it/podcast/la-checklist-per-il-tuo-portafoglio/)).
- **The source describes the rule for accumulation.** A later first-party explanation explicitly frames it as a rule for the accumulation phase, so DcaPal should not present it as a retirement or withdrawal glide path ([When does it make sense to change the portfolio?](https://www.thebull.it/podcast/quando-ha-senso-modificare-il-portafoglio-investimento-passivo-2-0/)).

The source does not specify an age rounding rule, an allowed age range, a minimum or maximum equity allocation, or what to do when the raw formula is below 0% or above 100%. Those are DcaPal guardrails, not The Bull facts. The implementation should make any guardrail explicit and should not describe a clamp or validation rule as part of the source formula.

### Age input

Use an integer age in completed years at the time the investor applies guidance. This matches the source's examples and keeps the stored input stable and understandable. The source does not justify fractional age, automatic birthday calculation, or a numeric minimum/maximum; those remain ordinary product validation choices.

The age is an input to the calculation, not a property of the Portfolio or Strategic allocation itself. It is therefore appropriate for it to appear in the applied guidance record while the current Portfolio remains a lean asset-level collection.

### Version and last-updated metadata

The first-party article has no formal framework version or changelog. Its machine-readable page metadata, observed on 2026-08-03, reported `datePublished` `2025-10-31` and `dateModified` `2026-01-08`; those are editorial page dates, not a version identifier. The page itself is the authoritative source for the rule as published, but it does not provide a stable `v1`/`v2` contract.

The first-party material also shows that the interpretation has evolved: older explanations referred to a Fed/BCE interest rate, while newer explanations prefer a risk-free rate that matches the portfolio horizon. DcaPal must therefore own the version boundary. Use `the-bull-v1` for the exact MVP interpretation documented here, including the selected short-rate series. If the formula, rate interpretation, or asset mapping changes, introduce a new framework version instead of changing the meaning of an existing applied record.

Keep the source URL and observed source metadata in the framework definition or release documentation. Do not add those fields to the applied guidance record, because the agreed persisted record contains only framework version, selected rate type, and age.

## Official rate sources

### Recommended MVP: overnight market rates

`fed_effr` and `ecb_estr` are the closest like-for-like choices for a product that must use a live Fed or ECB rate: both describe unsecured overnight money-market borrowing. They are still short-rate proxies, not the 10-year government yields that The Bull's newer explanation may prefer for a long-horizon portfolio. The selected rate type must make that choice visible.

#### Fed: Effective Federal Funds Rate (`fed_effr`)

The [New York Fed EFFR page](https://www.newyorkfed.org/markets/reference-rates/effr) defines EFFR as the volume-weighted median of overnight federal-funds transactions reported in FR 2420 data. The New York Fed publishes the prior business day's rate at about 09:00 ET. The [reference-rate methodology](https://www.newyorkfed.org/markets/reference-rates/additional-information-about-reference-rates) says the rate is rounded to the nearest basis point and describes the daily publication, revision, and contingency process.

Use the production endpoint documented by the [New York Fed Markets Data API](https://markets.newyorkfed.org/static/docs/markets-api.html):

```text
GET https://markets.newyorkfed.org/api/rates/unsecured/effr/last/1.json
```

The endpoint returns a `refRates` array. The live JSON payload observed on 2026-08-03 used these fields:

- `effectiveDate`: date of the underlying business-day observation.
- `type`: `EFFR`.
- `percentRate`: the rate in percent units, such as `3.63`, not a decimal fraction.
- `targetRateFrom` and `targetRateTo`: the associated FOMC target range, when supplied.
- `percentPercentile1`, `percentPercentile25`, `percentPercentile75`, `percentPercentile99`, and `volumeInBillions`: supporting market statistics.
- `revisionIndicator`: indicates a revised observation when present.

There is a source-contract quirk: the current API YAML schema calls the main field `percent`, while the live JSON response calls it `percentRate`. Treat `percentRate` as the observed production field, add a contract test when implementation begins, and keep this mismatch visible in the integration notes. The API documentation says production endpoints use the latest official service implementation, does not support multiple production versions, and was last updated 2026-06-12 when researched.

The New York Fed normally publishes EFFR on each non-holiday business day. It may revise a rate at about 14:30 ET when a same-day change exceeds one basis point. If the normal input is incomplete, it may use broker data; in extraordinary circumstances it may publish the prior day's rate. The published record carries footnote/revision information for those cases ([publication and revisions](https://www.newyorkfed.org/markets/reference-rates/additional-information-about-reference-rates)).

If the product wants the Fed policy stance rather than a market rate, the EFFR record's `targetRateFrom` and `targetRateTo` are available, and the FOMC sets the target range ([Federal Reserve H.15](https://www.federalreserve.gov/Releases/h15/)). That is a different selected rate type: it is a policy range, not EFFR. Do not silently replace EFFR with the range midpoint.

#### ECB: Euro short-term rate (`ecb_estr`)

The [ECB €STR overview](https://www.ecb.europa.eu/stats/financial_markets_and_interest_rates/euro_short-term_rate/html/eurostr_overview.en.html) defines €STR as the wholesale euro unsecured overnight borrowing cost of euro-area banks. It is based on transactions on the previous TARGET2 business day and is published at 08:00 CET on each TARGET2 business day. Its ISIN/benchmark item is `EU000A2X2A25`.

Use the ECB Data API series:

```text
GET https://data-api.ecb.europa.eu/service/data/EST/B.EU000A2X2A25.WT?format=csvdata&lastNObservations=1
```

The series key means daily business-week frequency (`B`), benchmark item `EU000A2X2A25`, and the volume-weighted trimmed-mean rate (`WT`). The [ECB Data Portal €STR metadata](https://data.ecb.europa.eu/data/datasets/EST/data-information) and the live CSV response expose these useful fields:

- `TIME_PERIOD`: the trade/reference date of the underlying transactions, not the later publication timestamp.
- `OBS_VALUE`: the rate in percent units.
- `OBS_STATUS` and `CONF_STATUS`: observation and confidentiality status.
- `FREQ`, `BENCHMARK_ITEM`, and `DATA_TYPE_EST`: series identity and frequency.
- `UNIT_MEASURE=PC`, `DECIMALS=3`, and `TIME_FORMAT=P1D`: unit and presentation metadata.

The ECB may revise and republish €STR once at 09:00 CET when an error changes the rate by more than two basis points. If no €STR is published by 09:00 CET through the official channels, the previous TARGET2 business day's rate applies, subject to the ECB's stated policy-rate adjustment. The ECB says it does not charge for €STR or license its use ([€STR overview](https://www.ecb.europa.eu/stats/financial_markets_and_interest_rates/euro_short-term_rate/html/eurostr_overview.en.html)).

#### Policy-rate alternatives: document, do not silently substitute

The ECB also publishes official policy-rate series in the `FM` dataflow:

| Selected series | ECB key | Term / meaning | Data behaviour |
| --- | --- | --- | --- |
| Deposit facility rate | `FM.B.U2.EUR.4F.KR.DFR.LEV` | Interest paid or charged on banks' overnight deposits with the Eurosystem | `FREQ=B`, but observations are dates of changes; carry the latest level forward |
| Main refinancing operations, fixed rate | `FM.B.U2.EUR.4F.KR.MRR_FR.LEV` | Cost for a one-week ECB refinancing operation | Same event-style series |
| Marginal lending facility | `FM.B.U2.EUR.4F.KR.MLFR.LEV` | Cost of overnight credit from the Eurosystem | Same event-style series |

The [ECB official-interest-rates methodology](https://data.ecb.europa.eu/methodology/official-interest-rates) says the Governing Council sets these three key rates and normally sets them every six weeks. The [ECB deposit-facility explainer](https://www.ecb.europa.eu/ecb-and-you/explainers/tell-me/html/what-is-the-deposit-facility-rate.en.html) describes the overnight deposit term and distinguishes it from the one-week MRO and overnight marginal lending facility. The raw series use `TIME_PERIOD` and `OBS_VALUE`, with `UNIT=PCPA` (percent per annum) and date-of-change values.

These policy rates are valid official alternatives if the product wants a monetary-policy stance, but they are not equivalent to €STR. For `the-bull-v1`, use €STR for the ECB short-rate option and keep a policy-rate option separate and explicitly named if it is added later.

### Source terms and reuse

The New York Fed [Terms of Use](https://www.newyorkfed.org/privacy/termsofuse) permit automated access, copying, storage, modification, and distribution subject to attribution, source identifiers, clear labelling of modifications, and a disclaimer that the New York Fed does not endorse the derived product. The terms also say that rates, calculation methods, schedules, revisions, and availability can change without notice. Any future UI or documentation that republishes EFFR should include the required attribution/disclaimer.

The ECB [reuse policy](https://www.ecb.europa.eu/stats/ecb_statistics/governance_and_quality_framework/html/usage_policy.en.html) permits free reuse of publicly available statistics if the source is quoted and the statistics and metadata are not modified; the [ECB disclaimer and copyright page](https://www.ecb.europa.eu/services/using-our-site/disclaimer/html/index.en.html) also disclaims liability for decisions based on published rates. This supports using the official data sources, but the product should identify them as source data and not as endorsement or personal financial advice.

## Apply-time contract and failure behaviour

The agreed product behaviour can be stated precisely:

1. The investor chooses `fed_effr` or `ecb_estr` and enters an integer age in completed years.
2. The application requests the latest available observation from that exact official series. “Latest available” means the latest source observation, so a prior business-day value is normal; it does not mean that the observation must have today's calendar date.
3. The application converts the source percentage to the formula's percentage-point input, calculates the raw equity and defensive percentages, applies any clearly documented DcaPal output guardrail, and shows the source/rate context during the interaction.
4. If the source is unavailable, returns no usable observation, or the payload cannot be parsed, guidance application fails closed. Do not switch silently to another central bank, another rate type, a cached rate, or a guessed value.
5. On success, persist only `framework_version`, `selected_rate_type`, and `age_years`. Do not persist the live rate, effective date, fetch time, or source metadata in the applied guidance record.

The official sources can revise same-day observations. Because the product intentionally does not store the rate value or observation date, a later re-application may produce a different result even with the same age and selected rate type. That is an expected consequence of live guidance and should be explained in product copy; it is not a reason to add the rate value to the agreed record.

## Sources

Primary sources used:

- [The Bull: Asset allocation and The Bull rule](https://www.thebull.it/podcast/asset-allocation-la-regola-di-the-bull/)
- [The Bull: How much should I invest in equities?](https://www.thebull.it/podcast/quanto-devo-investire-in-azioni-una-nuova-formula-e-file-da-scaricare/)
- [The Bull: When does it make sense to change the portfolio?](https://www.thebull.it/podcast/quando-ha-senso-modificare-il-portafoglio-investimento-passivo-2-0/)
- [The Bull checklist](https://www.thebull.it/podcast/la-checklist-per-il-tuo-portafoglio/)
- [New York Fed EFFR](https://www.newyorkfed.org/markets/reference-rates/effr)
- [New York Fed reference-rate methodology and publication details](https://www.newyorkfed.org/markets/reference-rates/additional-information-about-reference-rates)
- [New York Fed Markets Data API documentation](https://markets.newyorkfed.org/static/docs/markets-api.html)
- [Federal Reserve H.15 Selected Interest Rates](https://www.federalreserve.gov/Releases/h15/)
- [New York Fed Terms of Use](https://www.newyorkfed.org/privacy/termsofuse)
- [ECB €STR overview](https://www.ecb.europa.eu/stats/financial_markets_and_interest_rates/euro_short-term_rate/html/eurostr_overview.en.html)
- [ECB €STR dataset metadata](https://data.ecb.europa.eu/data/datasets/EST/data-information)
- [ECB Data API documentation](https://data.ecb.europa.eu/help/api/data)
- [ECB official interest rates methodology](https://data.ecb.europa.eu/methodology/official-interest-rates)
- [ECB key interest rates](https://data.ecb.europa.eu/key-figures/ecb-interest-rates-and-exchange-rates/key-ecb-interest-rates)
- [ECB policy on reuse of statistics](https://www.ecb.europa.eu/stats/ecb_statistics/governance_and_quality_framework/html/usage_policy.en.html)
- [ECB disclaimer and copyright](https://www.ecb.europa.eu/services/using-our-site/disclaimer/html/index.en.html)

