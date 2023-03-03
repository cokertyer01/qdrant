#![allow(deprecated)]

use std::num::{NonZeroU32, NonZeroU64};
use std::path::Path;
use std::sync::Arc;

use collection::collection::{Collection, RequestShardTransfer};
use collection::config::{CollectionConfig, CollectionParams, WalConfig};
use collection::operations::types::{CollectionError, VectorParams};
use collection::optimizers_builder::OptimizersConfig;
use collection::shards::channel_service::ChannelService;
use collection::shards::collection_shard_distribution::CollectionShardDistribution;
use collection::shards::replica_set::{ChangePeerState, ReplicaState};
use collection::shards::CollectionId;
use segment::types::Distance;

/// Test collections for this upper bound of shards.
/// Testing with more shards is problematic due to `number of open files problem`
/// See https://github.com/qdrant/qdrant/issues/379
#[allow(dead_code)]
pub const N_SHARDS: u32 = 3;

pub const TEST_OPTIMIZERS_CONFIG: OptimizersConfig = OptimizersConfig {
    deleted_threshold: 0.9,
    vacuum_min_vector_number: 1000,
    default_segment_number: 2,
    max_segment_size: None,
    memmap_threshold: None,
    indexing_threshold: 50_000,
    flush_interval_sec: 30,
    max_optimization_threads: 2,
};

#[cfg(test)]
#[allow(dead_code)]
pub async fn simple_collection_fixture(collection_path: &Path, shard_number: u32) -> Collection {
    let wal_config = WalConfig {
        wal_capacity_mb: 1,
        wal_segments_ahead: 0,
    };

    let collection_params = CollectionParams {
        vectors: VectorParams {
            size: NonZeroU64::new(4).unwrap(),
            distance: Distance::Dot,
        }
        .into(),
        shard_number: NonZeroU32::new(shard_number).expect("Shard number can not be zero"),
        replication_factor: NonZeroU32::new(1).unwrap(),
        write_consistency_factor: NonZeroU32::new(1).unwrap(),
        on_disk_payload: false,
    };

    let collection_config = CollectionConfig {
        params: collection_params,
        optimizer_config: TEST_OPTIMIZERS_CONFIG.clone(),
        wal_config,
        hnsw_config: Default::default(),
        quantization_config: Default::default(),
    };

    let snapshot_path = collection_path.join("snapshots");

    // Default to a collection with all the shards local
    new_local_collection(
        "test".to_string(),
        collection_path,
        &snapshot_path,
        &collection_config,
    )
    .await
    .unwrap()
}

pub fn dummy_on_replica_failure() -> ChangePeerState {
    Arc::new(move |_peer_id, _shard_id| {})
}

pub fn dummy_request_shard_transfer() -> RequestShardTransfer {
    Arc::new(move |_transfer| {})
}

/// Default to a collection with all the shards local
#[cfg(test)]
pub async fn new_local_collection(
    id: CollectionId,
    path: &Path,
    snapshots_path: &Path,
    config: &CollectionConfig,
) -> Result<Collection, CollectionError> {
    let collection = Collection::new(
        id,
        0,
        path,
        snapshots_path,
        config,
        CollectionShardDistribution::all_local(Some(config.params.shard_number.into()), 0),
        ChannelService::default(),
        dummy_on_replica_failure(),
        dummy_request_shard_transfer(),
        None,
        None,
    )
    .await;

    let collection = collection?;

    let local_shards = collection.get_local_shards().await;
    for shard_id in local_shards {
        collection
            .set_shard_replica_state(shard_id, 0, ReplicaState::Active, None)
            .await?;
    }
    Ok(collection)
}

/// Default to a collection with all the shards local
#[allow(dead_code)]
pub async fn load_local_collection(
    id: CollectionId,
    path: &Path,
    snapshots_path: &Path,
) -> Collection {
    Collection::load(
        id,
        0,
        path,
        snapshots_path,
        ChannelService::default(),
        dummy_on_replica_failure(),
        dummy_request_shard_transfer(),
        None,
        None,
    )
    .await
}
