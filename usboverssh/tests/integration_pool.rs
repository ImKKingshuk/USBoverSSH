// Integration tests for device pooling

use usboverssh::cache::DeviceListCache;
use usboverssh::pool::{DevicePool, PoolConfig, PoolManager};

#[tokio::test]
async fn test_pool_end_to_end_workflow() {
    let config = PoolConfig::default();
    let pool = DevicePool::new("test_pool".to_string(), config);

    // Reserve a device
    let reservation_id = pool
        .reserve_device("1-1".to_string(), "user1".to_string(), None)
        .await
        .unwrap();

    // Check pool status
    let status = pool.get_status().await;
    assert_eq!(status.active_reservations.len(), 1);
    assert_eq!(status.name, "test_pool");

    // Release the reservation
    pool.release_reservation(reservation_id).await.unwrap();

    // Verify device is no longer reserved
    let status = pool.get_status().await;
    assert_eq!(status.active_reservations.len(), 0);
}

#[tokio::test]
async fn test_pool_manager_multiple_pools() {
    let config = PoolConfig::default();
    let manager = PoolManager::new(config);

    // Create multiple pools
    let pool1 = manager.get_or_create_pool("pool1".to_string()).await;
    let pool2 = manager.get_or_create_pool("pool2".to_string()).await;

    // Reserve devices in different pools
    pool1.reserve_device("1-1".to_string(), "user1".to_string(), None)
        .await
        .unwrap();
    pool2.reserve_device("2-1".to_string(), "user2".to_string(), None)
        .await
        .unwrap();

    // Check all pool statuses
    let statuses = manager.get_all_statuses().await;
    assert_eq!(statuses.len(), 2);
}

#[tokio::test]
async fn test_cache_pool_interaction() {
    let cache = DeviceListCache::new(30);
    let config = PoolConfig::default();
    let pool = DevicePool::new("test_pool".to_string(), config);

    // Simulate caching a device list
    use usboverssh::device::{DeviceClass, DeviceInfo, DeviceSpeed};
    let device = DeviceInfo {
        bus_id: "1-1".to_string(),
        vendor_id: 0x1234,
        product_id: 0x5678,
        device_class: DeviceClass::Hid,
        bus_num: 1,
        dev_num: 1,
        speed: DeviceSpeed::High,
        manufacturer: Some("Test".to_string()),
        product: Some("Device".to_string()),
        serial: None,
        num_configurations: 1,
        is_attached: false,
        is_bound: false,
    };

    cache.set("test_key".to_string(), vec![device.clone()], None).await;

    // Retrieve from cache
    let cached = cache.get("test_key").await;
    assert!(cached.is_some());

    // Reserve the device from cache
    let cached_devices = cached.unwrap();
    let bus_id = &cached_devices[0].bus_id;
    pool.reserve_device(bus_id.clone(), "user1".to_string(), None)
        .await
        .unwrap();

    // Verify reservation
    let reservation = pool.get_reservation(bus_id).await;
    assert!(reservation.is_some());
}
