use randomon_settings::{MiscTweak, Settings};

use crate::error::Result;
use crate::handler::RomHandler;

pub fn apply_misc_tweaks<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let available = handler.misc_tweaks_available();
    for tweak in MiscTweak::in_application_order(available, settings.current_misc_tweaks) {
        handler.apply_misc_tweak(tweak)?;
    }
    Ok(())
}
