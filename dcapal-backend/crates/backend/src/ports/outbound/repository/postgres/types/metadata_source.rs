use num_enum::{IntoPrimitive, TryFromPrimitive};

/// The metadata-source codes persisted in `assets_data.metadata_source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, sqlx::Type)]
#[repr(i16)]
pub enum MetadataSource {
    /// Supplied by a client through v1 synchronization, or stored before
    /// provenance was tracked. The provider has not confirmed it.
    ClientSupplied = 0,
    /// Confirmed by the asset's provider.
    ProviderConfirmed = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_source_codes_round_trip() {
        // GIVEN each metadata source code, WHEN it converts to and from storage,
        // THEN it preserves the assigned value and rejects unsupported codes.
        assert_eq!(i16::from(MetadataSource::ClientSupplied), 0);
        assert_eq!(i16::from(MetadataSource::ProviderConfirmed), 1);
        assert_eq!(
            MetadataSource::try_from(0),
            Ok(MetadataSource::ClientSupplied)
        );
        assert_eq!(
            MetadataSource::try_from(1),
            Ok(MetadataSource::ProviderConfirmed)
        );
        assert!(MetadataSource::try_from(2).is_err());
    }
}
