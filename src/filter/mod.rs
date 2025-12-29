mod blend;
mod bounce;
mod glitch;
mod rainbow;

pub use blend::Blend;
pub use bounce::Bounce;
pub use glitch::Glitch;
pub use rainbow::Rainbow;

pub trait Filter {
    fn transform_buffer(
        &mut self,
        buffer: &mut Vec<crate::Pixel>,
        restore: &mut Option<Vec<crate::Pixel>>,
    );
}
