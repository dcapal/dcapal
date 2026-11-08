import * as React from "react";
import type { AssetClass, PortfolioAsset } from "../../fixtures/portfolio";
import type { AllocationMode } from "./invest-types";
import {
  allocationOnSwitchToStrategic,
  deriveAbsoluteWeights,
  sharesOnSwitchToStrategic,
  strategicFromAssets,
  withAssetShare,
  withClassTarget,
  withTargetWeights,
  type AssetShares,
  type ClassWeights,
  type StrategicAllocation,
} from "./strategic-allocation";

/** The Portfolio's allocation method together with its editable target weights. */
export interface PortfolioAllocationState {
  /** The Portfolio's allocation method. */
  method: AllocationMode;
  /** Assets whose target weight and drift follow the active method. */
  assets: PortfolioAsset[];
  /** Class targets and asset shares; kept in Simple mode so switching back is lossless. */
  allocation: StrategicAllocation;
  /**
   * The shares Strategic will really hold if the investor switches now: the
   * stored shares, or shares re-derived from the Simple edits made since.
   */
  sharesOnSwitch: AssetShares;
  /** Sets an asset's absolute target weight (Simple mode). */
  setAbsoluteWeight: (assetId: string, weight: number) => void;
  /** Sets an asset's relative target weight inside its class (Strategic mode). */
  setAssetShare: (assetId: string, share: number) => void;
  /** Sets one asset-class target weight (Strategic mode). */
  setClassWeight: (assetClass: AssetClass, weight: number) => void;
  /**
   * Switches to Strategic with the given class weights. Shares follow the rule
   * of `allocationOnSwitchToStrategic`: Simple edits are carried over, never
   * discarded.
   */
  switchToStrategic: (classTargets: ClassWeights) => void;
  /** Switches to Simple, keeping each asset's current absolute weight (class target x share). */
  switchToSimple: () => void;
}

/**
 * Owns the allocation method and target weights of a Portfolio's assets. In
 * Simple mode the absolute weights are stored; in Strategic mode class targets
 * and shares are stored and the absolute weights are derived.
 */
export function usePortfolioAllocation(
  baseAssets: readonly PortfolioAsset[],
  initialMethod: AllocationMode = "simple"
): PortfolioAllocationState {
  const [method, setMethod] = React.useState<AllocationMode>(initialMethod);
  const [allocation, setAllocation] = React.useState<StrategicAllocation>(() =>
    strategicFromAssets(baseAssets)
  );
  const [absolute, setAbsolute] = React.useState<Record<string, number>>(() =>
    Object.fromEntries(baseAssets.map((a) => [a.id, a.targetWeight]))
  );

  // Whether an absolute weight changed since the Strategic allocation was last
  // in force, in which case the stored shares no longer describe the weights.
  const [simpleEdited, setSimpleEdited] = React.useState(false);

  const assets = React.useMemo(
    () =>
      withTargetWeights(
        baseAssets,
        method === "strategic"
          ? deriveAbsoluteWeights(baseAssets, allocation)
          : absolute
      ),
    [absolute, allocation, baseAssets, method]
  );

  return {
    method,
    assets,
    allocation,
    sharesOnSwitch: sharesOnSwitchToStrategic(
      baseAssets,
      absolute,
      allocation,
      simpleEdited
    ),
    setAbsoluteWeight: (assetId, weight) => {
      setAbsolute((current) => ({ ...current, [assetId]: weight }));
      setSimpleEdited(true);
    },
    setAssetShare: (assetId, share) =>
      setAllocation((current) => withAssetShare(current, assetId, share)),
    setClassWeight: (assetClass, weight) =>
      setAllocation((current) => withClassTarget(current, assetClass, weight)),
    switchToStrategic: (classTargets) => {
      setAllocation((current) =>
        allocationOnSwitchToStrategic(
          baseAssets,
          absolute,
          current,
          classTargets,
          simpleEdited
        )
      );
      setSimpleEdited(false);
      setMethod("strategic");
    },
    switchToSimple: () => {
      if (method === "strategic") {
        setAbsolute(deriveAbsoluteWeights(baseAssets, allocation));
        setSimpleEdited(false);
      }
      setMethod("simple");
    },
  };
}
