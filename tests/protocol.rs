// Unit tests for USB/IP protocol

use usboverssh::protocol::{DeviceStatus, OpCode, USBIP_VERSION, UsbIpDeviceDescriptor, UsbIpHeader};
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

#[test]
fn test_opcode_to_u16() {
    assert_eq!(OpCode::ReqDevlist.to_u16(), 0x8005);
    assert_eq!(OpCode::RepDevlist.to_u16(), 0x0005);
    assert_eq!(OpCode::ReqImport.to_u16(), 0x8003);
    assert_eq!(OpCode::RepImport.to_u16(), 0x0003);
}

#[test]
fn test_opcode_from_u16() {
    assert_eq!(OpCode::from_u16(0x8005), Some(OpCode::ReqDevlist));
    assert_eq!(OpCode::from_u16(0x0005), Some(OpCode::RepDevlist));
    assert_eq!(OpCode::from_u16(0x8003), Some(OpCode::ReqImport));
    assert_eq!(OpCode::from_u16(0x9999), None);
}

#[test]
fn test_header_request() {
    let header = UsbIpHeader::request(OpCode::ReqDevlist);
    assert_eq!(header.version, USBIP_VERSION);
    assert_eq!(header.code, OpCode::ReqDevlist.to_u16());
    assert_eq!(header.status, 0);
}

#[test]
fn test_header_reply() {
    let header = UsbIpHeader::reply(OpCode::RepDevlist, 0);
    assert_eq!(header.version, USBIP_VERSION);
    assert_eq!(header.code, OpCode::RepDevlist.to_u16());
    assert_eq!(header.status, 0);
}

#[test]
fn test_device_status() {
    assert_eq!(DeviceStatus::Available as u32, 1);
    assert_eq!(DeviceStatus::Used as u32, 2);
    assert_eq!(DeviceStatus::Error as u32, 3);
}
