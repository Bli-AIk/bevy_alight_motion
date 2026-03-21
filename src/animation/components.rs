//! Animation components and resources for Alight Motion projects.

mod animated;
mod playback;
mod render;
mod runtime;

pub use animated::{AmAnimated, DEBUG_NEGATIVE_HEIGHT_SCALE};
pub use playback::AmPlayback;
pub use render::{
    AmCameraLayer, AmPathRepeat, AmSdfFillParams, AmSdfParams, AmSdfShapeParent, AmSdfStrokeParams,
};
pub use runtime::{
    AmEchoRuntime, AmRetimeInfo, AmUnifiedUsesTransformScale, EchoAlphaConfig, RetimeMode,
};
