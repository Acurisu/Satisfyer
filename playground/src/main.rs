use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::StreamExt;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

#[repr(u8)]
enum MOTORMODE {
    START = 0x1,
    STOP = 0x7,
}

const DINFO_SERVICE_UUID: Uuid = Uuid::from_u128(0x0000180a_0000_1000_8000_00805f9b34fb);
const MMODE_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(0x51361501_c5e7_47c7_8a6e_47ebc99d80e8);
const MVALUE_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(0x51361502_c5e7_47c7_8a6e_47ebc99d80e8);
const PATTERN: [(u64, u8); 80] = [(1058, 8), (352, 10), (1411, 12), (529, 25), (529, 16), (352, 25), (1411, 6), (1058, 8), (352, 10), (705, 12), (705, 10), (705, 16), (705, 25), (705, 16), (705, 12), (352, 0), (302, 12), (50, 0), (302, 12), (50, 0), (352, 12), (302, 25), (50, 0), (352, 25), (352, 16), (352, 12), (352, 0), (302, 25), (50, 0), (302, 25), (50, 0), (352, 25), (352, 16), (352, 25), (352, 16), (352, 12), (352, 0), (302, 12), (50, 0), (302, 12), (50, 0), (352, 12), (302, 25), (50, 0), (352, 25), (352, 16), (352, 12), (352, 0), (302, 25), (50, 0), (302, 25), (50, 0), (352, 25), (352, 0), (302, 80), (50, 0), (302, 80), (50, 0), (352, 80), (352, 0), (302, 12), (50, 0), (302, 12), (50, 0), (352, 12), (302, 25), (50, 0), (352, 25), (352, 16), (352, 12), (352, 0), (302, 33), (50, 0), (302, 33), (50, 0), (352, 33), (705, 16), (705, 58), (705, 25), (705, 80)];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    if adapters.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    let central = adapters.into_iter().nth(0).unwrap();
    let sf = find_sf(&central).await?.unwrap();

    sf.connect()
        .await
        .expect("Error connecting to BLE peripheral");
    sf.discover_services()
        .await
        .expect("Error discovering services");

    let chars = sf.characteristics();
    let motor_value = chars
        .iter()
        .find(|c| c.uuid == MVALUE_CHARACTERISTIC_UUID)
        .unwrap();
    let motor_mode = chars
        .iter()
        .find(|c| c.uuid == MMODE_CHARACTERISTIC_UUID)
        .unwrap();

    sf.write(
        &motor_mode,
        &[MOTORMODE::START as u8],
        WriteType::WithoutResponse,
    )
    .await?;

    for (du, st) in PATTERN {
        sf.write(
            &motor_value,
            &[st, st, st, st],
            WriteType::WithoutResponse,
        )
        .await?;
        time::sleep(Duration::from_millis(du)).await;
    }

    sf.write(
        &motor_mode,
        &[MOTORMODE::STOP as u8],
        WriteType::WithoutResponse,
    )
    .await?;

    sf.disconnect()
        .await
        .expect("Error disconnecting from BLE peripheral");
    Ok(())
}

async fn find_sf(central: &Adapter) -> Result<Option<Peripheral>, Box<dyn Error>> {
    let mut events = central.events().await?;
    central
        .start_scan(ScanFilter {
            services: vec![DINFO_SERVICE_UUID],
        })
        .await?;

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                let peripheral = central.peripheral(&id).await?;
                let properties = peripheral.properties().await?;
                let local_name = properties
                    .unwrap()
                    .local_name
                    .unwrap_or(String::from("Unknown"));
                println!("DeviceDiscovered: {} ({:?})", local_name, id);
                return Ok(Some(peripheral));
            }
            _ => {}
        }
    }

    Ok(None)
}
