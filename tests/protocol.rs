// Unit tests for USB/IP protocol

use usboverssh::protocol::{OpCode, USBIP_VERSION, UsbIpDeviceDescriptor, UsbIpHeader};
use usboverssh::device::{DeviceClass, DeviceSpeed, DeviceInfo};

#[test]
fn test_header_serialization() {
    let header = UsbIpHeader::request(OpCode::ReqDevlist);
    let bytes = header.to_bytes();
    let parsed = UsbIpHeader::from_bytes(&bytes);
    
    assert_eq!(parsed.version, USBIP_VERSION);
    assert_eq!(parsed.code, OpCode::ReqDevlist as u16);
}

#[test]
fn test_device_descriptor() {
    let device = DeviceInfo {
        bus_id: "1-1.2".to_string(),
        vendor_id: 0x1234,
        product_id: 0x5678,
        device_class: DeviceClass::Hid,
        bus_num: 1,
        dev_num: 3,
        speed: DeviceSpeed::High,
        manufacturer: Some("Test".to_string()),
        product: Some("Device".to_string()),
        serial: None,
        num_configurations: 1,
        is_attached: false,
        is_bound: false,
    };
    
    let desc = UsbIpDeviceDescriptor::from_device_info(&device);
    let bytes = desc.to_bytes();
    let parsed = UsbIpDeviceDescriptor::from_bytes(&bytes).unwrap();
    
    assert_eq!(parsed.bus_id, "1-1.2");
    assert_eq!(parsed.vendor_id, 0x1234);
    assert_eq!(parsed.product_id, 0x5678);
}
