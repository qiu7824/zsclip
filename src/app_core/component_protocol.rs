use super::{
    CommandQueue, ComponentId, LayoutInput, LayoutOutput, LifecycleEvent, Renderer, TextLayout,
    UiEvent,
};

pub(crate) trait Component {
    fn id(&self) -> ComponentId;
    fn lifecycle(&mut self, event: LifecycleEvent) {
        let _ = event;
    }
    fn update(&mut self, event: &UiEvent, commands: &mut CommandQueue);
    fn layout(&mut self, input: LayoutInput) -> LayoutOutput;
    fn render(&self, renderer: &mut dyn Renderer, text: &dyn TextLayout);
}
