//! KNXyz example: DPT codec.
//!
//! This example encodes KNX datapoint values into payload bytes and decodes them
//! back. The Rust examples package depends on the public `knxyz` facade. See
//! examples/README.md to run it locally.

use knxyz::dpt::{decode, encode, DptValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Scalar DPTs round-trip to a plain DptValue variant.
    let cases: [(&str, &str, DptValue); 5] = [
        ("1.001", "switch (boolean)", DptValue::Bool(true)),
        ("9.001", "temperature (degC, Float16)", DptValue::Temperature(21.0)),
        ("9.001", "temperature negative (degC)", DptValue::Temperature(-5.5)),
        ("5.010", "counter (raw 0-255)", DptValue::U8(128)),
        ("17.001", "scene number", DptValue::SceneNumber(7)),
    ];

    let mut ok = true;
    for (dpt, label, value) in &cases {
        let payload = encode(dpt, value.clone())?;
        let decoded = decode(dpt, &payload)?;
        let hex: Vec<String> = payload.iter().map(|b| format!("{b:02x}")).collect();
        let matched = &decoded == value;
        ok = ok && matched;
        println!(
            "DPT {:<7} {:<28} -> bytes=[{}] -> decoded={:?} {}",
            dpt,
            label,
            hex.join(" "),
            decoded,
            if matched { "OK" } else { "MISMATCH" }
        );
    }

    println!("DPT codec round-trip: {}", if ok { "all OK" } else { "FAILED" });
    if !ok {
        std::process::exit(1);
    }
    Ok(())
}
