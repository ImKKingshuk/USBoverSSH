// Integration tests for USB/IP protocol

use usboverssh::device::{DeviceClass, DeviceInfo, DeviceSpeed};
use usboverssh::protocol::{OpCode, UsbIpDeviceDescriptor, UsbIpHeader, USBIP_VERSION};

#[test]
fn test_usbip_header_serialization() {
    let header = UsbIpHeader::request(OpCode::ReqDevlist);
    let bytes = header.to_bytes();
    let parsed = UsbIpHeader::from_bytes(&bytes);

    assert_eq!(parsed.version, USBIP_VERSION);
    assert_eq!(parsed.code, OpCode::ReqDevlist.to_u16());
}

#[test]
fn test_usbip_device_descriptor() {
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
fn test_opcode_conversion() {
    assert_eq!(OpCode::ReqDevlist.to_u16(), 0x8005);
    assert_eq!(OpCode::RepDevlist.to_u16(), 0x0005);
    assert_eq!(OpCode::ReqImport.to_u16(), 0x8003);
    assert_eq!(OpCode::RepImport.to_u16(), 0x0003);

    assert_eq!(OpCode::from_u16(0x8005), Some(OpCode::ReqDevlist));
    assert_eq!(OpCode::from_u16(0x0005), Some(OpCode::RepDevlist));
    assert_eq!(OpCode::from_u16(0x9999), None);
}

#[test]
fn test_device_speed_conversion() {
    assert_eq!(DeviceSpeed::Low.to_usbip_speed(), 1);
    assert_eq!(DeviceSpeed::Full.to_usbip_speed(), 2);
    assert_eq!(DeviceSpeed::High.to_usbip_speed(), 3);
    assert_eq!(DeviceSpeed::Super.to_usbip_speed(), 5);
    assert_eq!(DeviceSpeed::SuperPlus.to_usbip_speed(), 6);
}

#[test]
fn test_device_class_from_code() {
    assert_eq!(DeviceClass::from_code(0x01), DeviceClass::Audio);
    assert_eq!(DeviceClass::from_code(0x08), DeviceClass::MassStorage);
    assert_eq!(DeviceClass::from_code(0x09), DeviceClass::Hub);
    assert_eq!(DeviceClass::from_code(0xff), DeviceClass::VendorSpecific);
}
