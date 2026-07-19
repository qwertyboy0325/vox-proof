#[cfg(feature = "persistence-spike")]
pub mod append_bundle;
#[cfg(feature = "persistence-spike")]
pub mod embedded_relational;
#[cfg(feature = "persistence-spike")]
pub mod fault;
#[cfg(feature = "persistence-spike")]
pub mod semantic_ops;

#[cfg(feature = "persistence-spike")]
pub use append_bundle::AppendBundleAdapter;
#[cfg(feature = "persistence-spike")]
pub use embedded_relational::EmbeddedRelationalAdapter;
