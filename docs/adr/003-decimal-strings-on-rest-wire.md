# ADR 003: Represent Rust decimal values as strings in REST JSON

DcaPal Backend stores portfolio quantities, prices, weights, and fee values as exact Rust `Decimal` values, while JavaScript JSON numbers become binary floating-point values in the browser. Serialize those decimal-backed REST fields as strings and keep the OpenAPI schemas as strings so the API does not discard precision at the transport boundary. The frontend converts values at the existing Redux and calculation boundary because the current optimizer and portfolio state use JavaScript numbers; introducing a decimal-math library is outside this migration. The market-price `price` field remains a JSON number because its backend type is `f64`, and the market-price timestamp remains epoch seconds because that is the current wire format.

The backend's existing float serializers and misleading timestamp schema must be corrected before Orval generation. This is an intentional wire-format correction for decimal response fields, while the client migration preserves the current product behavior and calculation model.

## Considered options

- Serialize decimal values as JSON numbers: rejected because JavaScript cannot preserve arbitrary decimal precision.
- Convert the whole frontend calculation model to decimal arithmetic now: rejected because it would expand this API migration into a portfolio and optimizer numeric-model migration.
- Keep the existing serializer/schema mismatch: rejected because generated types would describe a contract the server does not reliably provide.
