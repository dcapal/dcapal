/** Generated TanStack Query operations and OpenAPI model exports. */
export * from "./gen";

/** Request model used by the typed portfolio synchronization endpoint. */
export type {
  SyncPortfoliosRequest,
  SyncPortfoliosResponse,
  AllocationModeWire,
  FeeStructureV2,
  PortfolioAssetV2Request,
  PortfolioAssetV2Response,
  PortfolioV2Request,
  PortfolioV2Response,
  SharedAssetV2Request,
  SharedAssetV2Response,
  StrategicAllocationV2Request,
  StrategicAllocationV2Response,
  StrategicAssetClassTargetV2Request,
  StrategicAssetClassTargetV2Response,
  StrategicAssetTargetV2Request,
  StrategicAssetTargetV2Response,
  SyncPortfoliosV2Request,
  SyncPortfoliosV2Response,
  TransactionFeesV2Request,
  TransactionFeesV2Response,
  V2ErrorResponse,
  V2FieldError,
} from "./gen/model";

/** Hand-written client configuration and error APIs. */
export {
  ApiClientError,
  configureApiClientAuth,
  configureApiClientBaseUrl,
  resetApiClientConfiguration,
} from "./mutator/api-fetch";
