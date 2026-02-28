//! Icon rendering loop for tray icons.

mod data;

pub use data::Data;

/// A render task describing what to render.
pub struct Task {
    /// The text to display on the icon.
    pub text: String,

    /// When `true`, the icon is rendered with a red background to signal
    /// elevated attention.
    pub attention: bool,
}

/// Parameters for the render loop.
pub struct Params<RenderTaskReceiver, RenderedDataSender> {
    /// The width of the icon.
    pub width: u32,

    /// The height of the icon.
    pub height: u32,

    /// Function to receive render tasks.
    pub render_task_receiver: RenderTaskReceiver,

    /// Function to send rendered image data.
    pub rendered_data_sender: RenderedDataSender,
}

/// Run a blocking render loop that receives [`Task`]s and produces rendered
/// icon [`Data`].
pub fn run<RenderTaskReceiver, RenderedDataSender>(
    params: Params<RenderTaskReceiver, RenderedDataSender>,
) where
    RenderTaskReceiver: FnMut() -> Option<Task>,
    RenderedDataSender: FnMut(data::Data) -> std::ops::ControlFlow<()>,
{
    let Params {
        width,
        height,
        mut render_task_receiver,
        mut rendered_data_sender,
    } = params;
    let mut font_system = cosmic_text::FontSystem::new();
    let mut cache = cosmic_text::SwashCache::new();

    icon_render::load_font(font_system.db_mut());

    loop {
        let Some(task) = (render_task_receiver)() else {
            break;
        };

        let pixels = icon_render::render_text(
            &icon_render::RenderTextParams {
                text: &task.text,
                width,
                height,
                attention: task.attention,
            },
            &mut font_system,
            &mut cache,
        );

        let data = data::Data {
            pixels,
            width,
            height,
        };

        if (rendered_data_sender)(data).is_break() {
            break;
        }
    }
}
