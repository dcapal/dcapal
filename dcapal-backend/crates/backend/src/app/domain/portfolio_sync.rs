//! Storage-agnostic planning of a v2 Portfolio synchronization.
//!
//! A synchronization runs as "plan, then apply": the service loads a
//! [`SyncSnapshot`] of the user's stored Portfolios, [`plan_sync`] decides what
//! must be written and what the client is told (last-write-wins, tombstones,
//! echoed server-side Portfolios, allocation weights per mode), and the
//! persistence adapter applies the resulting [`SyncPlan`] atomically after
//! checking that the rows it was derived from have not changed meanwhile.
//! The adapter therefore makes no synchronization decisions.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::Timelike;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    DateTime,
    app::domain::{
        asset::AssetRef,
        portfolio::{
            AllocationMode, PortfolioAssetV2, PortfolioV2, StrategicAllocationV2,
            StrategicAssetClassTargetV2, StrategicAssetTargetV2, SyncPortfoliosV2Request,
            SyncPortfoliosV2Response, TransactionFees,
        },
    },
};

/// The token an optimistic concurrency check compares to detect that a stored
/// Portfolio changed after it was read.
///
/// It is the combination of all the columns that every writer touches: the
/// legacy v1 path never refreshes `updated_at` and only flips `deleted` on
/// deletion, while v2 always refreshes `updated_at` and `last_updated_at`, so
/// no single column is a reliable version on its own. The values are the
/// stored (microsecond precision) timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioVersion {
    /// Whether the Portfolio is a tombstone.
    pub deleted: bool,
    /// The client-supplied timestamp last stored for the Portfolio.
    pub last_updated_at: DateTime,
    /// The server-side timestamp of the last modification of the stored row.
    pub updated_at: DateTime,
}

/// One stored Portfolio Asset together with its stored allocation weight.
///
/// The weight is an absolute target in Simple mode and a relative target in
/// Strategic mode; which one it is depends on the Portfolio's mode.
/// `asset.target_weight` is not used: the weight lives in `weight`.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredAsset {
    pub asset: PortfolioAssetV2,
    pub weight: Decimal,
}

/// The stored content of a live (not deleted) Portfolio, as persisted.
///
/// This is the neutral stored shape: it keeps every stored value regardless
/// of the Allocation Mode, and [`StoredPortfolio::to_portfolio`] projects it
/// to the aggregate clients see.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredAggregate {
    pub name: String,
    pub quote_currency: String,
    pub allocation_mode: AllocationMode,
    pub drift_band: Option<Decimal>,
    pub fees: Option<TransactionFees>,
    /// Whether a Strategic Allocation object (possibly empty) was submitted.
    pub strategic_allocation_present: bool,
    pub assets: Vec<StoredAsset>,
    /// Stored Strategic Asset Class targets, kept even in Simple mode.
    pub class_targets: Vec<StrategicAssetClassTargetV2>,
}

/// A Portfolio of the synchronizing user as it is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredPortfolio {
    pub id: Uuid,
    pub version: PortfolioVersion,
    /// The stored content, absent exactly when the Portfolio is a tombstone.
    pub aggregate: Option<StoredAggregate>,
}

impl StoredPortfolio {
    /// Tells whether the stored Portfolio is a tombstone.
    pub fn is_deleted(&self) -> bool {
        self.version.deleted
    }

    /// Projects the stored content to the aggregate returned to clients, or
    /// `None` for a tombstone.
    pub fn to_portfolio(&self) -> Option<PortfolioV2> {
        let aggregate = self.aggregate.as_ref()?;
        Some(project(Projection {
            id: self.id,
            name: aggregate.name.clone(),
            quote_currency: aggregate.quote_currency.clone(),
            allocation_mode: aggregate.allocation_mode,
            drift_band: aggregate.drift_band,
            fees: aggregate.fees.clone(),
            last_updated_at: self.version.last_updated_at,
            strategic_allocation_present: aggregate.strategic_allocation_present,
            assets: aggregate.assets.clone(),
            class_targets: aggregate.class_targets.clone(),
        }))
    }
}

/// All Portfolios (including tombstones) of one user, ordered by identifier.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncSnapshot {
    pub portfolios: Vec<StoredPortfolio>,
}

/// Whether a written Portfolio is created or updated in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteKind {
    /// No stored row exists for the identifier: insert one.
    Create,
    /// The user's live row exists: replace its content.
    Update,
}

/// The target weight of one Portfolio Asset, derived for the Portfolio's mode.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetAllocation {
    pub asset: AssetRef,
    /// Absolute target in Simple mode, relative target in Strategic mode.
    pub weight: Decimal,
}

/// A validated Portfolio aggregate to persist with everything derived from it.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioWrite {
    pub kind: WriteKind,
    /// The submitted aggregate, with the timestamp reduced to the precision
    /// the store keeps.
    pub portfolio: PortfolioV2,
    /// The allocation weight of every asset of `portfolio`, in its order.
    pub allocations: Vec<AssetAllocation>,
    /// Whether a Strategic Allocation object was submitted.
    pub strategic_allocation_present: bool,
    /// The Strategic Asset Class targets to store, whatever the mode is.
    pub class_targets: Vec<StrategicAssetClassTargetV2>,
}

impl PortfolioWrite {
    fn new(kind: WriteKind, submitted: &PortfolioV2) -> Self {
        let mut portfolio = submitted.clone();
        portfolio.last_updated_at = store_precision(portfolio.last_updated_at);
        let relative_targets: HashMap<&AssetRef, Decimal> = submitted
            .strategic_allocation
            .iter()
            .flat_map(|allocation| &allocation.asset_targets)
            .map(|target| (&target.asset, target.relative_target_weight))
            .collect();
        let allocations = submitted
            .assets
            .iter()
            .map(|asset| AssetAllocation {
                asset: asset.asset.clone(),
                weight: match submitted.allocation_mode {
                    AllocationMode::Simple => asset.target_weight.unwrap_or_default(),
                    AllocationMode::Strategic => relative_targets
                        .get(&asset.asset)
                        .copied()
                        .unwrap_or_default(),
                },
            })
            .collect();
        let class_targets = submitted
            .strategic_allocation
            .as_ref()
            .map(|allocation| allocation.asset_class_targets.clone())
            .unwrap_or_default();
        Self {
            kind,
            strategic_allocation_present: submitted.strategic_allocation.is_some(),
            portfolio,
            allocations,
            class_targets,
        }
    }

    /// Returns the aggregate clients see once this write is stored.
    pub fn to_response(&self) -> PortfolioV2 {
        let weights: HashMap<&AssetRef, Decimal> = self
            .allocations
            .iter()
            .map(|allocation| (&allocation.asset, allocation.weight))
            .collect();
        project(Projection {
            id: self.portfolio.id,
            name: self.portfolio.name.clone(),
            quote_currency: self.portfolio.quote_currency.clone(),
            allocation_mode: self.portfolio.allocation_mode,
            drift_band: self.portfolio.drift_band,
            fees: self.portfolio.fees.clone(),
            last_updated_at: self.portfolio.last_updated_at,
            strategic_allocation_present: self.strategic_allocation_present,
            assets: self
                .portfolio
                .assets
                .iter()
                .map(|asset| StoredAsset {
                    asset: asset.clone(),
                    weight: weights.get(&asset.asset).copied().unwrap_or_default(),
                })
                .collect(),
            class_targets: self.class_targets.clone(),
        })
    }
}

/// How a requested deletion is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TombstoneKind {
    /// The identifier is unknown: store a new tombstone row owned by the user.
    Create,
    /// The user's row exists (or is created by the same plan): mark it deleted.
    Mark,
}

/// A terminal soft deletion to store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tombstone {
    pub id: Uuid,
    pub kind: TombstoneKind,
}

/// The stored state a plan was derived from for one Portfolio identifier.
///
/// The adapter must verify it still holds before applying the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedPortfolio {
    pub id: Uuid,
    /// The version read from the snapshot, or `None` when no row of the user
    /// existed and none must exist when the plan is applied.
    pub version: Option<PortfolioVersion>,
}

/// Everything needed to apply a synchronization and answer the client.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncPlan {
    /// Portfolios to create or replace, in request order.
    pub writes: Vec<PortfolioWrite>,
    /// Deletions to store, without duplicates, in request order.
    pub tombstones: Vec<Tombstone>,
    /// The state the plan relies on: every identifier of the snapshot, of the
    /// submitted Portfolios and of the requested deletions, ordered by
    /// identifier.
    pub expected: Vec<ExpectedPortfolio>,
    /// What the client is told once the plan has been applied.
    pub response: SyncPortfoliosV2Response,
}

/// Plans the synchronization of `request` against the stored `snapshot`.
///
/// The rules, applied per Portfolio:
/// - a stored tombstone is terminal: the client is told it is deleted and
///   nothing is written;
/// - a stored Portfolio whose `last_updated_at` is not older than the
///   submitted one wins: it is returned unchanged and nothing is written;
/// - otherwise (unknown, or older on the server) the submitted aggregate is
///   written and returned as it will be stored;
/// - stored Portfolios the request does not mention are returned (tombstones
///   as deleted identifiers);
/// - every requested deletion becomes a tombstone and is acknowledged once.
pub fn plan_sync(snapshot: &SyncSnapshot, request: &SyncPortfoliosV2Request) -> SyncPlan {
    let stored: HashMap<Uuid, &StoredPortfolio> = snapshot
        .portfolios
        .iter()
        .map(|portfolio| (portfolio.id, portfolio))
        .collect();
    let incoming_ids: HashSet<Uuid> = request
        .portfolios
        .iter()
        .map(|portfolio| portfolio.id)
        .collect();
    let requested_deleted_ids: HashSet<Uuid> = request.deleted_portfolios.iter().copied().collect();

    let mut writes = Vec::new();
    let mut updated_portfolios = Vec::new();
    let mut deleted_portfolios = Vec::new();
    for submitted in &request.portfolios {
        match stored.get(&submitted.id) {
            Some(current) if !is_written(submitted, current) => {
                if current.is_deleted() {
                    deleted_portfolios.push(submitted.id);
                } else {
                    updated_portfolios.extend(current.to_portfolio());
                }
            }
            current => {
                let kind = if current.is_some() {
                    WriteKind::Update
                } else {
                    WriteKind::Create
                };
                let write = PortfolioWrite::new(kind, submitted);
                updated_portfolios.push(write.to_response());
                writes.push(write);
            }
        }
    }

    // Return server-side aggregates omitted by the client.
    for current in &snapshot.portfolios {
        if incoming_ids.contains(&current.id) || requested_deleted_ids.contains(&current.id) {
            continue;
        }
        if current.is_deleted() {
            deleted_portfolios.push(current.id);
        } else {
            updated_portfolios.extend(current.to_portfolio());
        }
    }

    let created_ids: HashSet<Uuid> = writes
        .iter()
        .filter(|write| write.kind == WriteKind::Create)
        .map(|write| write.portfolio.id)
        .collect();
    let mut acknowledged: HashSet<Uuid> = deleted_portfolios.iter().copied().collect();
    let mut tombstones = Vec::new();
    let mut tombstoned = HashSet::new();
    for deleted_id in &request.deleted_portfolios {
        if tombstoned.insert(*deleted_id) {
            let exists = stored.contains_key(deleted_id) || created_ids.contains(deleted_id);
            tombstones.push(Tombstone {
                id: *deleted_id,
                kind: if exists {
                    TombstoneKind::Mark
                } else {
                    TombstoneKind::Create
                },
            });
        }
        if acknowledged.insert(*deleted_id) {
            deleted_portfolios.push(*deleted_id);
        }
    }

    let mut expected: BTreeMap<Uuid, Option<PortfolioVersion>> = snapshot
        .portfolios
        .iter()
        .map(|portfolio| (portfolio.id, Some(portfolio.version.clone())))
        .collect();
    for id in incoming_ids.iter().chain(&requested_deleted_ids) {
        expected.entry(*id).or_insert(None);
    }

    SyncPlan {
        writes,
        tombstones,
        expected: expected
            .into_iter()
            .map(|(id, version)| ExpectedPortfolio { id, version })
            .collect(),
        response: SyncPortfoliosV2Response {
            updated_portfolios,
            deleted_portfolios,
        },
    }
}

/// Tells whether a submitted Portfolio must be written: the stored one is not
/// a tombstone and is strictly older than the submitted one.
fn is_written(submitted: &PortfolioV2, current: &StoredPortfolio) -> bool {
    !current.is_deleted() && current.version.last_updated_at < submitted.last_updated_at
}

/// Reduces a timestamp to the microsecond precision of the store, so that the
/// response of a write equals what a later read returns.
fn store_precision(timestamp: DateTime) -> DateTime {
    timestamp
        .with_nanosecond(timestamp.nanosecond() / 1_000 * 1_000)
        .unwrap_or(timestamp)
}

/// The values from which the client-facing aggregate is projected.
struct Projection {
    id: Uuid,
    name: String,
    quote_currency: String,
    allocation_mode: AllocationMode,
    drift_band: Option<Decimal>,
    fees: Option<TransactionFees>,
    last_updated_at: DateTime,
    strategic_allocation_present: bool,
    assets: Vec<StoredAsset>,
    class_targets: Vec<StrategicAssetClassTargetV2>,
}

/// Builds the aggregate clients see from stored values.
///
/// Assets are ordered by provider and symbol and class targets by class. In
/// Simple mode each asset carries its absolute target and a present Strategic
/// Allocation is empty; in Strategic mode assets carry no own target and the
/// Strategic Allocation lists the class targets and the relative target of
/// every asset.
fn project(mut parts: Projection) -> PortfolioV2 {
    parts.assets.sort_by(|a, b| {
        (a.asset.asset.provider.as_str(), &a.asset.asset.symbol)
            .cmp(&(b.asset.asset.provider.as_str(), &b.asset.asset.symbol))
    });
    parts.class_targets.sort_by_key(|target| target.asset_class);
    let simple = parts.allocation_mode == AllocationMode::Simple;

    let strategic_allocation = parts
        .strategic_allocation_present
        .then(|| StrategicAllocationV2 {
            asset_class_targets: if simple {
                Vec::new()
            } else {
                parts.class_targets.clone()
            },
            asset_targets: if simple {
                Vec::new()
            } else {
                parts
                    .assets
                    .iter()
                    .map(|asset| StrategicAssetTargetV2 {
                        asset: asset.asset.asset.clone(),
                        relative_target_weight: asset.weight,
                    })
                    .collect()
            },
        });
    let assets = parts
        .assets
        .into_iter()
        .map(|stored| PortfolioAssetV2 {
            target_weight: simple.then_some(stored.weight),
            ..stored.asset
        })
        .collect();

    PortfolioV2 {
        id: parts.id,
        name: parts.name,
        quote_currency: parts.quote_currency,
        allocation_mode: parts.allocation_mode,
        drift_band: parts.drift_band,
        fees: parts.fees,
        assets,
        strategic_allocation,
        last_updated_at: parts.last_updated_at,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use rust_decimal::dec;

    use super::*;
    use crate::app::domain::portfolio::{AssetClass, FeeStructure, Provider};

    const A: Uuid = Uuid::from_u128(0xA);
    const B: Uuid = Uuid::from_u128(0xB);
    const C: Uuid = Uuid::from_u128(0xC);

    fn at(day: u32) -> DateTime {
        Utc.with_ymd_and_hms(2026, 1, day, 0, 0, 0).unwrap()
    }

    fn version(day: u32, deleted: bool) -> PortfolioVersion {
        PortfolioVersion {
            deleted,
            last_updated_at: at(day),
            updated_at: at(day),
        }
    }

    fn holding(provider: Provider, symbol: &str) -> PortfolioAssetV2 {
        PortfolioAssetV2 {
            asset: AssetRef::new(provider, symbol),
            asset_class_override: None,
            quantity: dec!(2.50),
            target_weight: None,
            manual_price: Some(dec!(10.10)),
            average_buy_price: None,
            fees: None,
        }
    }

    /// A Simple aggregate whose assets carry the given absolute targets.
    fn simple(id: Uuid, day: u32, assets: Vec<(PortfolioAssetV2, Option<Decimal>)>) -> PortfolioV2 {
        PortfolioV2 {
            id,
            name: format!("portfolio {id}"),
            quote_currency: "EUR".to_string(),
            allocation_mode: AllocationMode::Simple,
            drift_band: None,
            fees: None,
            assets: assets
                .into_iter()
                .map(|(asset, weight)| PortfolioAssetV2 {
                    target_weight: weight,
                    ..asset
                })
                .collect(),
            strategic_allocation: None,
            last_updated_at: at(day),
        }
    }

    /// Stores `portfolio` as the snapshot would hold its projection.
    fn stored(portfolio: &PortfolioV2, version: PortfolioVersion) -> StoredPortfolio {
        let write = PortfolioWrite::new(WriteKind::Update, portfolio);
        let weights: HashMap<&AssetRef, Decimal> = write
            .allocations
            .iter()
            .map(|allocation| (&allocation.asset, allocation.weight))
            .collect();
        StoredPortfolio {
            id: portfolio.id,
            version,
            aggregate: Some(StoredAggregate {
                name: portfolio.name.clone(),
                quote_currency: portfolio.quote_currency.clone(),
                allocation_mode: portfolio.allocation_mode,
                drift_band: portfolio.drift_band,
                fees: portfolio.fees.clone(),
                strategic_allocation_present: write.strategic_allocation_present,
                assets: portfolio
                    .assets
                    .iter()
                    .map(|asset| StoredAsset {
                        asset: PortfolioAssetV2 {
                            target_weight: None,
                            ..asset.clone()
                        },
                        weight: weights[&asset.asset],
                    })
                    .collect(),
                class_targets: write.class_targets,
            }),
        }
    }

    fn tombstone(id: Uuid, day: u32) -> StoredPortfolio {
        StoredPortfolio {
            id,
            version: version(day, true),
            aggregate: None,
        }
    }

    fn request(
        portfolios: Vec<PortfolioV2>,
        deleted_portfolios: Vec<Uuid>,
    ) -> SyncPortfoliosV2Request {
        SyncPortfoliosV2Request {
            schema_version: 2,
            portfolios,
            deleted_portfolios,
        }
    }

    fn strategic(
        id: Uuid,
        day: u32,
        assets: Vec<PortfolioAssetV2>,
        class_targets: Vec<(AssetClass, Decimal)>,
        asset_targets: Vec<(&AssetRef, Decimal)>,
    ) -> PortfolioV2 {
        PortfolioV2 {
            allocation_mode: AllocationMode::Strategic,
            assets,
            strategic_allocation: Some(StrategicAllocationV2 {
                asset_class_targets: class_targets
                    .into_iter()
                    .map(|(asset_class, target_weight)| StrategicAssetClassTargetV2 {
                        asset_class,
                        target_weight,
                    })
                    .collect(),
                asset_targets: asset_targets
                    .into_iter()
                    .map(|(asset, relative_target_weight)| StrategicAssetTargetV2 {
                        asset: asset.clone(),
                        relative_target_weight,
                    })
                    .collect(),
            }),
            ..simple(id, day, Vec::new())
        }
    }

    #[test]
    fn unknown_portfolio_is_created_and_returned_as_stored() {
        // GIVEN an empty snapshot and a request with one new Portfolio,
        // WHEN the sync is planned,
        // THEN the Portfolio is written as a creation, expected to be absent,
        // and returned with its asset target.
        let submitted = simple(A, 5, vec![(holding(Provider::YF, "vwce"), Some(dec!(100)))]);

        let plan = plan_sync(
            &SyncSnapshot::default(),
            &request(vec![submitted.clone()], vec![]),
        );

        assert_eq!(plan.writes.len(), 1);
        assert_eq!(plan.writes[0].kind, WriteKind::Create);
        assert_eq!(plan.writes[0].portfolio, submitted);
        assert_eq!(
            plan.expected,
            vec![ExpectedPortfolio {
                id: A,
                version: None
            }]
        );
        assert_eq!(plan.response.updated_portfolios, vec![submitted]);
        assert!(plan.response.deleted_portfolios.is_empty());
        assert!(plan.tombstones.is_empty());
    }

    #[test]
    fn newer_client_portfolio_replaces_the_stored_one() {
        // GIVEN a stored Portfolio older than the submitted one,
        // WHEN the sync is planned,
        // THEN it is written as an update expecting the stored version and the
        // submitted content is returned.
        let old = simple(A, 1, vec![(holding(Provider::YF, "VWCE"), Some(dec!(1)))]);
        let submitted = simple(A, 2, vec![(holding(Provider::YF, "VWCE"), Some(dec!(9)))]);
        let snapshot = SyncSnapshot {
            portfolios: vec![stored(&old, version(1, false))],
        };

        let plan = plan_sync(&snapshot, &request(vec![submitted.clone()], vec![]));

        assert_eq!(plan.writes[0].kind, WriteKind::Update);
        assert_eq!(plan.expected[0].version, Some(version(1, false)));
        assert_eq!(plan.response.updated_portfolios, vec![submitted]);
    }

    #[test]
    fn stored_portfolio_that_is_not_older_wins_and_is_not_written() {
        // GIVEN stored Portfolios that are newer than / as new as the submitted
        // ones,
        // WHEN the sync is planned,
        // THEN nothing is written and the server copies are returned.
        let newer = simple(A, 5, vec![(holding(Provider::YF, "VWCE"), Some(dec!(1)))]);
        let same = simple(B, 3, vec![]);
        let snapshot = SyncSnapshot {
            portfolios: vec![
                stored(&newer, version(5, false)),
                stored(&same, version(3, false)),
            ],
        };
        let mut client_a = newer.clone();
        client_a.name = "client".to_string();
        client_a.last_updated_at = at(2);
        let mut client_b = same.clone();
        client_b.name = "client".to_string();

        let plan = plan_sync(&snapshot, &request(vec![client_a, client_b], vec![]));

        assert!(plan.writes.is_empty());
        assert_eq!(plan.response.updated_portfolios, vec![newer, same]);
    }

    #[test]
    fn stored_tombstone_is_terminal_for_submitted_portfolios() {
        // GIVEN a stored tombstone and a newer submitted Portfolio with its id,
        // WHEN the sync is planned,
        // THEN the Portfolio is not written and its id is reported deleted.
        let snapshot = SyncSnapshot {
            portfolios: vec![tombstone(A, 1)],
        };

        let plan = plan_sync(&snapshot, &request(vec![simple(A, 9, vec![])], vec![]));

        assert!(plan.writes.is_empty());
        assert!(plan.response.updated_portfolios.is_empty());
        assert_eq!(plan.response.deleted_portfolios, vec![A]);
    }

    #[test]
    fn server_only_portfolios_are_echoed_in_snapshot_order() {
        // GIVEN a snapshot with a live Portfolio, a tombstone and another live
        // Portfolio that the request does not mention,
        // WHEN the sync is planned,
        // THEN live ones are returned as aggregates in snapshot order and the
        // tombstone as a deleted identifier, with nothing written.
        let first = simple(A, 1, vec![]);
        let second = simple(C, 1, vec![]);
        let snapshot = SyncSnapshot {
            portfolios: vec![
                stored(&first, version(1, false)),
                tombstone(B, 1),
                stored(&second, version(1, false)),
            ],
        };

        let plan = plan_sync(&snapshot, &request(vec![], vec![]));

        assert!(plan.writes.is_empty() && plan.tombstones.is_empty());
        assert_eq!(plan.response.updated_portfolios, vec![first, second]);
        assert_eq!(plan.response.deleted_portfolios, vec![B]);
        assert_eq!(
            plan.expected.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![A, B, C]
        );
    }

    #[test]
    fn deletion_of_an_unknown_id_creates_a_tombstone() {
        // GIVEN an empty snapshot and a request deleting an unknown id twice,
        // WHEN the sync is planned,
        // THEN one creating tombstone is planned, expected absent, and the id
        // is acknowledged once.
        let plan = plan_sync(&SyncSnapshot::default(), &request(vec![], vec![A, A]));

        assert_eq!(
            plan.tombstones,
            vec![Tombstone {
                id: A,
                kind: TombstoneKind::Create
            }]
        );
        assert_eq!(plan.expected[0].version, None);
        assert_eq!(plan.response.deleted_portfolios, vec![A]);
    }

    #[test]
    fn deletion_of_a_stored_id_marks_it_and_is_not_echoed() {
        // GIVEN a stored live Portfolio and a stored tombstone that the request
        // asks to delete,
        // WHEN the sync is planned,
        // THEN both are marked, neither is echoed as an aggregate, and each id
        // is acknowledged exactly once in request order.
        let live = simple(A, 1, vec![]);
        let snapshot = SyncSnapshot {
            portfolios: vec![stored(&live, version(1, false)), tombstone(B, 1)],
        };

        let plan = plan_sync(&snapshot, &request(vec![], vec![B, A]));

        assert_eq!(
            plan.tombstones,
            vec![
                Tombstone {
                    id: B,
                    kind: TombstoneKind::Mark
                },
                Tombstone {
                    id: A,
                    kind: TombstoneKind::Mark
                }
            ]
        );
        assert!(plan.response.updated_portfolios.is_empty());
        assert_eq!(plan.response.deleted_portfolios, vec![B, A]);
    }

    #[test]
    fn deletions_and_server_tombstones_are_acknowledged_without_duplicates() {
        // GIVEN a stored tombstone omitted by the request and a deletion of an
        // unknown id,
        // WHEN the sync is planned,
        // THEN the server tombstone comes first and each id appears once.
        let snapshot = SyncSnapshot {
            portfolios: vec![tombstone(A, 1)],
        };

        let plan = plan_sync(&snapshot, &request(vec![], vec![C]));

        assert_eq!(plan.response.deleted_portfolios, vec![A, C]);
    }

    #[test]
    fn simple_mode_defaults_missing_targets_to_zero() {
        // GIVEN a Simple aggregate with one asset without target,
        // WHEN the sync is planned,
        // THEN the derived allocation is zero and the response carries it.
        let submitted = simple(A, 1, vec![(holding(Provider::YF, "VWCE"), None)]);

        let plan = plan_sync(&SyncSnapshot::default(), &request(vec![submitted], vec![]));

        assert_eq!(plan.writes[0].allocations[0].weight, Decimal::ZERO);
        assert_eq!(
            plan.response.updated_portfolios[0].assets[0].target_weight,
            Some(Decimal::ZERO)
        );
    }

    #[test]
    fn strategic_mode_derives_relative_weights_per_asset() {
        // GIVEN a Strategic aggregate whose targets cover only one of two assets
        // and list class targets out of order,
        // WHEN the sync is planned,
        // THEN assets carry no own target, every asset (sorted by provider and
        // symbol) has a relative target defaulting to zero, class targets are
        // sorted by class, and the stored allocation follows the same weights.
        let vwce = holding(Provider::YF, "VWCE");
        let cash = holding(Provider::YF, "CASH");
        let submitted = strategic(
            A,
            1,
            vec![vwce.clone(), cash.clone()],
            vec![
                (AssetClass::Cash, dec!(40.0)),
                (AssetClass::Equity, dec!(60.0)),
            ],
            vec![(&vwce.asset, dec!(100.00))],
        );

        let plan = plan_sync(&SyncSnapshot::default(), &request(vec![submitted], vec![]));

        let weights: Vec<_> = plan.writes[0]
            .allocations
            .iter()
            .map(|allocation| (allocation.asset.symbol.as_str(), allocation.weight))
            .collect();
        assert_eq!(weights, vec![("VWCE", dec!(100.00)), ("CASH", dec!(0))]);
        let response = &plan.response.updated_portfolios[0];
        assert!(response.assets.iter().all(|a| a.target_weight.is_none()));
        let symbols: Vec<_> = response.assets.iter().map(|a| &a.asset.symbol).collect();
        assert_eq!(symbols, ["CASH", "VWCE"]);
        let allocation = response.strategic_allocation.as_ref().unwrap();
        assert_eq!(
            allocation
                .asset_targets
                .iter()
                .map(|t| (t.asset.symbol.as_str(), t.relative_target_weight))
                .collect::<Vec<_>>(),
            vec![("CASH", dec!(0)), ("VWCE", dec!(100.00))]
        );
        assert_eq!(
            allocation
                .asset_class_targets
                .iter()
                .map(|t| (t.asset_class, t.target_weight))
                .collect::<Vec<_>>(),
            vec![
                (AssetClass::Equity, dec!(60.0)),
                (AssetClass::Cash, dec!(40.0))
            ]
        );
    }

    #[test]
    fn strategic_mode_without_a_strategic_allocation_has_none_in_the_response() {
        // GIVEN a Strategic aggregate submitted with a null Strategic Allocation,
        // WHEN the sync is planned,
        // THEN the response keeps it null and the assets get zero weights.
        let mut submitted = strategic(
            A,
            1,
            vec![holding(Provider::YF, "VWCE")],
            Vec::new(),
            Vec::new(),
        );
        submitted.strategic_allocation = None;

        let plan = plan_sync(&SyncSnapshot::default(), &request(vec![submitted], vec![]));

        assert!(!plan.writes[0].strategic_allocation_present);
        assert_eq!(plan.writes[0].allocations[0].weight, Decimal::ZERO);
        assert_eq!(
            plan.response.updated_portfolios[0].strategic_allocation,
            None
        );
    }

    #[test]
    fn simple_mode_keeps_class_targets_to_store_but_hides_them() {
        // GIVEN a Simple aggregate that carries a Strategic Allocation with a
        // class target,
        // WHEN the sync is planned,
        // THEN the target is stored, and the response shows an empty Strategic
        // Allocation object.
        let mut submitted = simple(A, 1, vec![(holding(Provider::YF, "VWCE"), Some(dec!(50)))]);
        submitted.strategic_allocation = Some(StrategicAllocationV2 {
            asset_class_targets: vec![StrategicAssetClassTargetV2 {
                asset_class: AssetClass::Equity,
                target_weight: dec!(100),
            }],
            asset_targets: Vec::new(),
        });

        let plan = plan_sync(&SyncSnapshot::default(), &request(vec![submitted], vec![]));

        assert_eq!(plan.writes[0].class_targets.len(), 1);
        assert_eq!(
            plan.response.updated_portfolios[0].strategic_allocation,
            Some(StrategicAllocationV2 {
                asset_class_targets: Vec::new(),
                asset_targets: Vec::new(),
            })
        );
    }

    #[test]
    fn assets_are_ordered_by_provider_name_then_symbol() {
        // GIVEN assets of both providers submitted in reverse order,
        // WHEN the sync is planned,
        // THEN the response orders them by provider identifier ("kraken" before
        // "yf") and symbol, while the write keeps the submitted order.
        let submitted = simple(
            A,
            1,
            vec![
                (holding(Provider::YF, "ZZZ"), None),
                (holding(Provider::YF, "AAA"), None),
                (holding(Provider::Kraken, "BTC"), None),
            ],
        );

        let plan = plan_sync(&SyncSnapshot::default(), &request(vec![submitted], vec![]));

        let ordered: Vec<_> = plan.response.updated_portfolios[0]
            .assets
            .iter()
            .map(|a| (a.asset.provider.as_str(), a.asset.symbol.as_str()))
            .collect();
        assert_eq!(ordered, [("kraken", "BTC"), ("yf", "AAA"), ("yf", "ZZZ")]);
        assert_eq!(plan.writes[0].portfolio.assets[0].asset.symbol, "ZZZ");
    }

    #[test]
    fn class_override_decimals_and_fees_are_returned_verbatim() {
        // GIVEN an asset whose override equals its class, decimals with trailing
        // zeros and a variable fee,
        // WHEN the sync is planned,
        // THEN the response returns them unchanged.
        let mut asset = holding(Provider::YF, "VWCE");
        asset.asset_class_override = Some(AssetClass::Equity);
        asset.fees = Some(TransactionFees {
            max_fee_impact: Some(dec!(1.50)),
            fee_structure: FeeStructure::Variable {
                fee_rate: dec!(0.250),
                min_fee: Some(dec!(1.00)),
                max_fee: None,
            },
        });
        let submitted = simple(A, 1, vec![(asset.clone(), Some(dec!(100.00)))]);

        let plan = plan_sync(&SyncSnapshot::default(), &request(vec![submitted], vec![]));

        let returned = &plan.response.updated_portfolios[0].assets[0];
        assert_eq!(returned.asset_class_override, Some(AssetClass::Equity));
        assert_eq!(returned.quantity.to_string(), "2.50");
        assert_eq!(returned.target_weight.unwrap().to_string(), "100.00");
        assert_eq!(returned.fees, asset.fees);
    }

    #[test]
    fn written_timestamp_is_reduced_to_store_precision() {
        // GIVEN a submitted timestamp with nanoseconds,
        // WHEN the sync is planned,
        // THEN the written and returned timestamp keep only microseconds, as a
        // later read would, while the comparison used the submitted value.
        let mut submitted = simple(A, 2, vec![]);
        submitted.last_updated_at = at(2) + Duration::nanoseconds(123_456_789);

        let plan = plan_sync(&SyncSnapshot::default(), &request(vec![submitted], vec![]));

        let expected = at(2) + Duration::nanoseconds(123_456_000);
        assert_eq!(plan.writes[0].portfolio.last_updated_at, expected);
        assert_eq!(
            plan.response.updated_portfolios[0].last_updated_at,
            expected
        );
    }

    #[test]
    fn a_stored_copy_equal_to_the_truncated_timestamp_is_older_than_the_submitted_one() {
        // GIVEN a stored timestamp equal to the truncation of the submitted one,
        // WHEN the sync is planned,
        // THEN the submitted Portfolio is treated as newer (raw comparison).
        let mut submitted = simple(A, 2, vec![]);
        submitted.last_updated_at = at(2) + Duration::nanoseconds(1_500);
        let snapshot = SyncSnapshot {
            portfolios: vec![stored(&simple(A, 2, vec![]), version(2, false))],
        };

        let plan = plan_sync(&snapshot, &request(vec![submitted], vec![]));

        assert_eq!(plan.writes.len(), 1);
    }

    #[test]
    fn expected_covers_snapshot_request_and_deleted_ids_once_in_order() {
        // GIVEN a snapshot with B, a submitted Portfolio A and a deletion of C,
        // WHEN the sync is planned,
        // THEN the plan expects A and C absent and B at its stored version,
        // ordered by identifier.
        let existing = simple(B, 1, vec![]);
        let snapshot = SyncSnapshot {
            portfolios: vec![stored(&existing, version(1, false))],
        };

        let plan = plan_sync(&snapshot, &request(vec![simple(A, 1, vec![])], vec![C]));

        assert_eq!(
            plan.expected,
            vec![
                ExpectedPortfolio {
                    id: A,
                    version: None
                },
                ExpectedPortfolio {
                    id: B,
                    version: Some(version(1, false))
                },
                ExpectedPortfolio {
                    id: C,
                    version: None
                },
            ]
        );
    }

    #[test]
    fn asset_class_order_is_the_canonical_class_target_order() {
        // GIVEN the Asset Classes,
        // WHEN they are compared,
        // THEN the order is the stored class-code order used for responses.
        assert!(AssetClass::Other < AssetClass::Equity);
        assert!(AssetClass::Equity < AssetClass::Bond);
        assert!(AssetClass::Bond < AssetClass::Cash);
        assert!(AssetClass::Cash < AssetClass::Crypto);
        assert!(AssetClass::Crypto < AssetClass::Commodity);
    }
}
