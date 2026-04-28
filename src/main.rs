use ddc::{Ddc, DdcHost as _, FeatureCode};
use ddc_hi::Display;
use std::{borrow::Cow, collections::HashMap, error::Error, future::pending};
use zbus::{connection, interface};

use ddc_brightness_daemon::BrightnessChange;

const LUMINANCE_FEATURE_CODE: FeatureCode = 0x10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    tracing::info!("Enumerating displays");
    let displays = Display::enumerate();

    tracing::info!("Detected displays:");
    for (i, disp) in displays.iter().enumerate() {
        tracing::info!(
            "Display {i}: {}",
            disp.info.model_name.as_deref().unwrap_or("Unknown Model"),
        );
    }

    let state = State { displays };

    let _conn = connection::Builder::session()?
        .name("org.tritoke.Brightness1")?
        .serve_at("/org/tritoke/Displays", state)?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}

struct State {
    displays: Vec<Display>,
}

#[interface(name = "org.tritoke.Displays")]
impl State {
    fn get_display_metadata(&self) -> Vec<HashMap<&'static str, Cow<'_, str>>> {
        tracing::debug!("[get_display_metadata]");
        self.displays.iter().map(build_metadata).collect()
    }

    fn set_absolute(&mut self, monitor: usize, amount: u16) -> Result<(), zbus::fdo::Error> {
        tracing::debug!("[set_absolute] display={monitor}, amount={amount}");

        let Some(disp) = self.displays.get_mut(monitor) else {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "No such display {monitor}"
            )));
        };

        change_brightness(monitor, disp, BrightnessChange::Absolute(amount))
    }

    fn change_relative(&mut self, monitor: usize, amount: i16) -> Result<(), zbus::fdo::Error> {
        tracing::debug!("[change_relative] display={monitor}, amount={amount}");

        let Some(disp) = self.displays.get_mut(monitor) else {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "No such display {monitor}"
            )));
        };

        change_brightness(monitor, disp, BrightnessChange::Relative(amount))
    }

    fn list_brightness(&mut self, monitor: usize) -> Result<u16, zbus::fdo::Error> {
        tracing::debug!("[list_brightness] monitor={monitor}");

        let Some(disp) = self.displays.get_mut(monitor) else {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "No such display {monitor}"
            )));
        };

        get_brightness(monitor, disp)
    }
}

fn change_brightness(
    display_no: usize,
    display: &mut Display,
    change: BrightnessChange,
) -> Result<(), zbus::fdo::Error> {
    tracing::debug!("[change_brightness] display={display_no}, change={change:?}");

    if matches!(change, BrightnessChange::Relative(0)) {
        return Ok(());
    }

    let model = display
        .info
        .model_name
        .as_deref()
        .unwrap_or("Unknown Model");

    let disp = format!("display {display_no} ({model})");

    let Ok(vcp) = display.handle.get_vcp_feature(LUMINANCE_FEATURE_CODE) else {
        let msg = format!("Timed out waiting for response from {disp}");
        tracing::warn!("{msg}");
        return Err(zbus::fdo::Error::Timeout(msg));
    };
    let old_value = vcp.value();
    display.handle.sleep();

    let new_value = change.apply(old_value);
    if old_value == new_value {
        tracing::info!("No change needed for {disp}");
        return Ok(());
    }

    tracing::info!("Changing brighness of {disp} from {old_value} to {new_value}");
    if let Err(e) = display
        .handle
        .set_vcp_feature(LUMINANCE_FEATURE_CODE, new_value)
    {
        let msg = format!("Failed to set brightness for {disp}: {e}");
        tracing::error!("{msg}");
        return Err(zbus::fdo::Error::Failed(msg));
    }
    display.handle.sleep();

    Ok(())
}

fn get_brightness(display_no: usize, display: &mut Display) -> Result<u16, zbus::fdo::Error> {
    let model = display
        .info
        .model_name
        .as_deref()
        .unwrap_or("Unknown Model");

    let disp = format!("display {display_no} ({model})");

    let Ok(vcp) = display.handle.get_vcp_feature(LUMINANCE_FEATURE_CODE) else {
        let msg = format!("Timed out waiting for response from {disp}");
        tracing::warn!("{msg}");
        return Err(zbus::fdo::Error::Timeout(msg));
    };
    let value = vcp.value();
    display.handle.sleep();

    Ok(value)
}

fn build_metadata(display: &Display) -> HashMap<&'static str, Cow<'_, str>> {
    let mut metadata = HashMap::new();

    metadata.insert(
        "model_name",
        Cow::Borrowed(
            display
                .info
                .model_name
                .as_deref()
                .unwrap_or("Unknown Model"),
        ),
    );
    metadata.insert(
        "manufacturer_id",
        Cow::Borrowed(display.info.manufacturer_id.as_deref().unwrap_or("???")),
    );
    metadata.insert(
        "model_id",
        display
            .info
            .model_id
            .map(|num| Cow::Owned(format!("{num:04X}")))
            .unwrap_or(Cow::Borrowed("????")),
    );
    metadata.insert(
        "serial",
        display
            .info
            .serial
            .map(|num| Cow::Owned(format!("{num:08X}")))
            .unwrap_or(Cow::Borrowed("????")),
    );
    metadata.insert(
        "manufacture_week",
        display
            .info
            .manufacture_week
            .map(|num| Cow::Owned(format!("{num}")))
            .unwrap_or(Cow::Borrowed("??")),
    );
    metadata.insert(
        "manufacture_year",
        display
            .info
            .manufacture_year
            .map(|num| Cow::Owned(format!("{}", 1990 + num as u16)))
            .unwrap_or(Cow::Borrowed("??")),
    );

    metadata
}
