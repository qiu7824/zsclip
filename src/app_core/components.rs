use super::{
    CommandQueue, Component, ComponentId, LayoutInput, LayoutNode, LayoutOutput, LayoutProtocol,
    LifecycleEvent, LifecycleState, Renderer, TextLayout, TextStyle, UiEvent,
};

pub(crate) struct Label {
    id: ComponentId,
    lifecycle: LifecycleState,
    text: String,
    style: TextStyle,
    bounds: super::Rect,
}

impl Label {
    pub(crate) fn new(id: ComponentId, text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            id,
            lifecycle: LifecycleState::new(),
            text: text.into(),
            style,
            bounds: super::Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
        }
    }

    pub(crate) fn lifecycle_state(&self) -> LifecycleState {
        self.lifecycle
    }

    pub(crate) fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub(crate) fn set_style(&mut self, style: TextStyle) {
        self.style = style;
    }
}

impl LayoutProtocol for Label {
    fn layout(&mut self, input: LayoutInput) -> LayoutOutput {
        self.bounds = input.bounds;
        LayoutOutput {
            bounds: self.bounds,
            children: vec![LayoutNode {
                component: self.id,
                bounds: self.bounds,
            }],
        }
    }
}

impl Component for Label {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn lifecycle(&mut self, event: LifecycleEvent) {
        self.lifecycle.apply(event);
    }

    fn update(&mut self, _event: &UiEvent, _commands: &mut CommandQueue) {}

    fn layout(&mut self, input: LayoutInput) -> LayoutOutput {
        LayoutProtocol::layout(self, input)
    }

    fn render(&self, renderer: &mut dyn Renderer, text: &dyn TextLayout) {
        for run in text.layout_runs(&self.text, &self.style, self.bounds) {
            renderer.draw_text(&run, &self.style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::{
        Color, HorizontalAlign, Rect, Size, TextRun, TextWeight, TextWrap, VerticalAlign,
    };

    #[derive(Default)]
    struct RecordingRenderer {
        text: Vec<String>,
    }

    impl Renderer for RecordingRenderer {
        fn fill_rect(&mut self, _rect: Rect, _color: Color) {}
        fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: i32) {}
        fn draw_text(&mut self, run: &TextRun, _style: &TextStyle) {
            self.text.push(run.text.clone());
        }
        fn push_clip(&mut self, _rect: Rect) {}
        fn pop_clip(&mut self) {}
    }

    struct SingleRunTextLayout;

    impl TextLayout for SingleRunTextLayout {
        fn measure(&self, text: &str, _style: &TextStyle, _max_width: i32) -> Size {
            Size {
                width: text.len() as i32,
                height: 1,
            }
        }

        fn layout_runs(&self, text: &str, _style: &TextStyle, bounds: Rect) -> Vec<TextRun> {
            vec![TextRun {
                text: text.to_string(),
                bounds,
            }]
        }
    }

    fn style() -> TextStyle {
        TextStyle {
            font_family: "Test".to_string(),
            size: 14.0,
            weight: TextWeight::Regular,
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            horizontal_align: HorizontalAlign::Start,
            vertical_align: VerticalAlign::Center,
            wrap: TextWrap::NoWrap,
            ellipsis: true,
        }
    }

    #[test]
    fn label_uses_layout_and_renderer_protocols() {
        let mut label = Label::new(ComponentId(1), "hello", style());
        label.lifecycle(LifecycleEvent::Mount);
        label.lifecycle(LifecycleEvent::Resume);
        let bounds = Rect {
            x: 10,
            y: 20,
            width: 100,
            height: 30,
        };
        let output = Component::layout(&mut label, LayoutInput { bounds, scale: 1.0 });
        let mut renderer = RecordingRenderer::default();
        label.render(&mut renderer, &SingleRunTextLayout);

        assert_eq!(output.bounds, bounds);
        assert_eq!(output.children[0].component, ComponentId(1));
        assert_eq!(renderer.text, vec!["hello"]);
        assert_eq!(
            label.lifecycle_state().phase(),
            super::super::ComponentPhase::Active
        );
    }
}
