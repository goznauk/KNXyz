use knx_core::{GroupAddress, IndividualAddress, KnxError};

#[test]
fn individual_address_roundtrips() {
    let address: IndividualAddress = "1.1.4".parse().unwrap();
    assert_eq!(address.to_string(), "1.1.4");
    assert_eq!(address.raw(), 0x1104);
}

#[test]
fn group_address_three_level_roundtrips() {
    let address: GroupAddress = "1/2/3".parse().unwrap();
    assert_eq!(address.to_string(), "1/2/3");
    assert_eq!(address.raw(), 0x0a03);
}

#[test]
fn group_address_two_level_roundtrips() {
    let address = GroupAddress::parse_two_level("1/515").unwrap();
    assert_eq!(address.to_two_level_display().to_string(), "1/515");
    assert_eq!(address.raw(), 0x0a03);
}

#[test]
fn invalid_addresses_are_rejected() {
    assert!("16.0.0".parse::<IndividualAddress>().is_err());
    assert!("1/8/0".parse::<GroupAddress>().is_err());
    assert!(GroupAddress::parse_two_level("32/0").is_err());
}

// CHARACTERIZATION: boundary maxima all pack to raw 0xFFFF and round-trip.

#[test]
fn individual_address_boundary_max_packs_to_ffff_and_roundtrips() {
    let address: IndividualAddress = "15.15.255".parse().unwrap();
    assert_eq!(address.raw(), 0xFFFF);
    assert_eq!(address.to_string(), "15.15.255");

    let reparsed: IndividualAddress = address.to_string().parse().unwrap();
    assert_eq!(reparsed, address);

    let from_raw = IndividualAddress::from_raw(0xFFFF);
    assert_eq!(from_raw.area(), 15);
    assert_eq!(from_raw.line(), 15);
    assert_eq!(from_raw.device(), 255);
}

#[test]
fn group_address_three_level_boundary_max_packs_to_ffff_and_roundtrips() {
    let address: GroupAddress = "31/7/255".parse().unwrap();
    assert_eq!(address.raw(), 0xFFFF);
    assert_eq!(address.to_string(), "31/7/255");

    let reparsed: GroupAddress = address.to_string().parse().unwrap();
    assert_eq!(reparsed, address);

    let from_raw = GroupAddress::from_raw(0xFFFF);
    assert_eq!(from_raw.main(), 31);
    assert_eq!(from_raw.middle(), 7);
    assert_eq!(from_raw.sub(), 255);
}

#[test]
fn group_address_two_level_boundary_max_packs_to_ffff_and_roundtrips() {
    let address = GroupAddress::parse_two_level("31/2047").unwrap();
    assert_eq!(address.raw(), 0xFFFF);
    assert_eq!(address.to_two_level_display().to_string(), "31/2047");

    let two_level = address.to_two_level_display().to_string();
    let reparsed = GroupAddress::parse_two_level(&two_level).unwrap();
    assert_eq!(reparsed, address);

    let from_raw = GroupAddress::from_raw(0xFFFF);
    assert_eq!(from_raw.two_level_sub(), 0x07FF);
    assert_eq!(from_raw.two_level_sub(), 2047);
}

#[test]
fn from_raw_does_no_validation_and_preserves_bits() {
    for x in [0x0000u16, 0x1104, 0x0A03, 0xFFFF] {
        assert_eq!(IndividualAddress::from_raw(x).raw(), x);
        assert_eq!(GroupAddress::from_raw(x).raw(), x);
    }
}

#[test]
fn accessors_decompose_known_raw_values() {
    let ind = IndividualAddress::from_raw(0x1104);
    assert_eq!(ind.area(), 1);
    assert_eq!(ind.line(), 1);
    assert_eq!(ind.device(), 4);

    let grp = GroupAddress::from_raw(0x0A03);
    assert_eq!(grp.main(), 1);
    assert_eq!(grp.middle(), 2);
    assert_eq!(grp.sub(), 3);
    assert_eq!(grp.two_level_sub(), 0x0A03 & 0x07FF);

    // two-level new(1, 515) packs to the same raw as 1/2/3.
    let two = GroupAddress::new_two_level(1, 515).unwrap();
    assert_eq!(two.raw(), 0x0A03);
}

#[test]
fn two_level_display_uses_two_parts() {
    let grp = GroupAddress::from_raw(0x0A03);
    assert_eq!(grp.to_two_level_display().to_string(), "1/515");
}

#[test]
fn individual_address_new_range_errors() {
    assert_eq!(
        IndividualAddress::new(0x10, 0, 0),
        Err(KnxError::InvalidAddress("individual area out of range"))
    );
    assert_eq!(
        IndividualAddress::new(0, 0x10, 0),
        Err(KnxError::InvalidAddress("individual line out of range"))
    );
    // device is u8 with no range check.
    assert!(IndividualAddress::new(15, 15, 255).is_ok());
}

#[test]
fn group_address_new_three_level_range_errors() {
    assert_eq!(
        GroupAddress::new_three_level(0x20, 0, 0),
        Err(KnxError::InvalidAddress("group main out of range"))
    );
    assert_eq!(
        GroupAddress::new_three_level(0, 0x08, 0),
        Err(KnxError::InvalidAddress("group middle out of range"))
    );
    // sub is u8 with no range check.
    assert!(GroupAddress::new_three_level(31, 7, 255).is_ok());
}

#[test]
fn group_address_new_two_level_range_errors() {
    assert_eq!(
        GroupAddress::new_two_level(0x20, 0),
        Err(KnxError::InvalidAddress("group main out of range"))
    );
    assert_eq!(
        GroupAddress::new_two_level(0, 0x0800),
        Err(KnxError::InvalidAddress("group sub out of range"))
    );
    assert!(GroupAddress::new_two_level(31, 0x07FF).is_ok());
}

#[test]
fn individual_address_fromstr_missing_part_errors() {
    assert_eq!(
        "".parse::<IndividualAddress>(),
        Err(KnxError::InvalidAddress("invalid numeric address part"))
    );
    assert_eq!(
        "1".parse::<IndividualAddress>(),
        Err(KnxError::InvalidAddress("missing individual line"))
    );
    assert_eq!(
        "1.1".parse::<IndividualAddress>(),
        Err(KnxError::InvalidAddress("missing individual device"))
    );
    assert_eq!(
        "1.1.4.5".parse::<IndividualAddress>(),
        Err(KnxError::InvalidAddress("too many individual parts"))
    );
}

#[test]
fn group_address_fromstr_missing_part_errors() {
    assert_eq!(
        "1".parse::<GroupAddress>(),
        Err(KnxError::InvalidAddress("missing group middle"))
    );
    assert_eq!(
        "1/2".parse::<GroupAddress>(),
        Err(KnxError::InvalidAddress("missing group sub"))
    );
    assert_eq!(
        "1/2/3/4".parse::<GroupAddress>(),
        Err(KnxError::InvalidAddress("too many group parts"))
    );
}

#[test]
fn parse_two_level_missing_part_errors() {
    assert_eq!(
        GroupAddress::parse_two_level("1"),
        Err(KnxError::InvalidAddress("missing group sub"))
    );
    assert_eq!(
        GroupAddress::parse_two_level("1/2/3"),
        Err(KnxError::InvalidAddress("too many group parts"))
    );
}

#[test]
fn non_numeric_and_overflow_parts_report_invalid_numeric() {
    assert_eq!(
        "x.0.0".parse::<IndividualAddress>(),
        Err(KnxError::InvalidAddress("invalid numeric address part"))
    );
    assert_eq!(
        "..".parse::<IndividualAddress>(),
        Err(KnxError::InvalidAddress("invalid numeric address part"))
    );
    // 256 overflows the u8 field; parse fails before new() so we get the
    // numeric-parse error, NOT the range message.
    assert_eq!(
        "256.0.0".parse::<IndividualAddress>(),
        Err(KnxError::InvalidAddress("invalid numeric address part"))
    );
    assert_eq!(
        "0/0/256".parse::<GroupAddress>(),
        Err(KnxError::InvalidAddress("invalid numeric address part"))
    );
    // parse_two_level sub is parsed as u16; overflow still reports numeric.
    assert_eq!(
        GroupAddress::parse_two_level("0/70000"),
        Err(KnxError::InvalidAddress("invalid numeric address part"))
    );
}
