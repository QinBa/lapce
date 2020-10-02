use std::f64::INFINITY;

use druid::kurbo::{Point, Rect, Size, Vec2};
use druid::{
    scroll_component::*, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx,
    LifeCycle, LifeCycleCtx, PaintCtx, UpdateCtx, Widget, WidgetPod,
};

use crate::command::{LapceUICommand, LAPCE_UI_COMMAND};
use crate::state::LAPCE_STATE;

#[derive(Debug, Clone)]
enum ScrollDirection {
    Bidirectional,
    Vertical,
    Horizontal,
}

/// A container that scrolls its contents.
///
/// This container holds a single child, and uses the wheel to scroll it
/// when the child's bounds are larger than the viewport.
///
/// The child is laid out with completely unconstrained layout bounds by
/// default. Restrict to a specific axis with [`vertical`] or [`horizontal`].
/// When restricted to scrolling on a specific axis the child's size is
/// locked on the opposite axis.
///
/// [`vertical`]: struct.Scroll.html#method.vertical
/// [`horizontal`]: struct.Scroll.html#method.horizontal
pub struct LapceScroll<T, W> {
    child: WidgetPod<T, W>,
    scroll_component: ScrollComponent,
    direction: ScrollDirection,
}

impl<T: Data, W: Widget<T>> LapceScroll<T, W> {
    /// Create a new scroll container.
    ///
    /// This method will allow scrolling in all directions if child's bounds
    /// are larger than the viewport. Use [vertical](#method.vertical) and
    /// [horizontal](#method.horizontal) methods to limit scrolling to a specific axis.
    pub fn new(child: W) -> LapceScroll<T, W> {
        LapceScroll {
            child: WidgetPod::new(child),
            scroll_component: ScrollComponent::new(),
            direction: ScrollDirection::Bidirectional,
        }
    }

    /// Restrict scrolling to the vertical axis while locking child width.
    pub fn vertical(mut self) -> Self {
        self.direction = ScrollDirection::Vertical;
        self
    }

    /// Restrict scrolling to the horizontal axis while locking child height.
    pub fn horizontal(mut self) -> Self {
        self.direction = ScrollDirection::Horizontal;
        self
    }

    /// Returns a reference to the child widget.
    pub fn child(&self) -> &W {
        self.child.widget()
    }

    /// Returns a mutable reference to the child widget.
    pub fn child_mut(&mut self) -> &mut W {
        self.child.widget_mut()
    }

    /// Returns the size of the child widget.
    pub fn child_size(&self) -> Size {
        self.scroll_component.content_size
    }

    /// Returns the current scroll offset.
    pub fn offset(&self) -> Vec2 {
        self.scroll_component.scroll_offset
    }

    pub fn scroll(&mut self, x: f64, y: f64) {
        let mut offset = self.offset();
        offset.x = offset.x + x;
        offset.y = offset.y + y;
        if offset.y < 0.0 {
            offset.y = 0.0;
        }
        self.scroll_component.scroll_offset = offset;
        self.child.set_viewport_offset(offset);
    }

    pub fn scroll_to(&mut self, x: f64, y: f64) {
        let offset = Vec2::new(x, y);
        self.scroll_component.scroll_offset = offset;
        self.child.set_viewport_offset(offset);
    }

    pub fn ensure_visible(
        &mut self,
        scroll_size: Size,
        rect: &Rect,
        margin: &(f64, f64),
    ) -> bool {
        let mut new_offset = self.offset();
        let content_size = self.scroll_component.content_size;

        let (x_margin, y_margin) = margin;

        new_offset.x = if new_offset.x < rect.x1 + x_margin - scroll_size.width
        {
            (rect.x1 + x_margin - scroll_size.width)
                .min(content_size.width - scroll_size.width)
        } else if new_offset.x > rect.x0 - x_margin {
            (rect.x0 - x_margin).max(0.0)
        } else {
            new_offset.x
        };

        new_offset.y = if new_offset.y < rect.y1 + y_margin - scroll_size.height
        {
            (rect.y1 + y_margin - scroll_size.height)
                .min(content_size.height - scroll_size.height)
        } else if new_offset.y > rect.y0 - y_margin {
            (rect.y0 - y_margin).max(0.0)
        } else {
            new_offset.y
        };

        if new_offset == self.offset() {
            return false;
        }

        self.scroll_component.scroll_offset = new_offset;
        self.child.set_viewport_offset(new_offset);
        true
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for LapceScroll<T, W> {
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut T,
        env: &Env,
    ) {
        match event {
            Event::Internal(_) => {
                self.child.event(ctx, event, data, env);
            }
            Event::Command(cmd) => match cmd {
                _ if cmd.is(LAPCE_UI_COMMAND) => {
                    let command = cmd.get_unchecked(LAPCE_UI_COMMAND);
                    match command {
                        LapceUICommand::RequestLayout => {
                            println!("scroll request layout");
                            ctx.request_layout();
                        }
                        LapceUICommand::RequestPaint => {
                            println!("scroll request paint");
                            ctx.request_paint();
                        }
                        LapceUICommand::EnsureVisible((rect, margin)) => {
                            if self.ensure_visible(ctx.size(), rect, margin) {
                                ctx.request_paint();
                            }
                            return;
                        }
                        LapceUICommand::ScrollTo((x, y)) => {
                            self.scroll_to(*x, *y);
                            return;
                        }
                        LapceUICommand::Scroll((x, y)) => {
                            self.scroll(*x, *y);
                            ctx.request_paint();
                            return;
                        }
                        _ => println!(
                            "scroll unprocessed ui command {:?}",
                            command
                        ),
                    }
                }
                _ => (),
            },
            _ => (),
        };
        // self.scroll_component.event(ctx, event, env);
        if !ctx.is_handled() {
            let viewport = Rect::from_origin_size(Point::ORIGIN, ctx.size());

            let force_event = self.child.is_hot() || self.child.is_active();
            let child_event = event.transform_scroll(
                self.scroll_component.scroll_offset,
                viewport,
                force_event,
            );
            if let Some(child_event) = child_event {
                self.child.event(ctx, &child_event, data, env);
            };
        }

        self.scroll_component.handle_scroll(ctx, event, env);
        // In order to ensure that invalidation regions are correctly propagated up the tree,
        // we need to set the viewport offset on our child whenever we change our scroll offset.
        self.child.set_viewport_offset(self.offset());
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        self.scroll_component.lifecycle(ctx, event, env);
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: &T,
        data: &T,
        env: &Env,
    ) {
        self.child.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Scroll");

        let max_bc = match self.direction {
            ScrollDirection::Bidirectional => Size::new(INFINITY, INFINITY),
            ScrollDirection::Vertical => Size::new(bc.max().width, INFINITY),
            ScrollDirection::Horizontal => Size::new(INFINITY, bc.max().height),
        };

        let child_bc = BoxConstraints::new(Size::ZERO, max_bc);
        let child_size = self.child.layout(ctx, &child_bc, data, env);
        self.scroll_component.content_size = child_size;
        self.child
            .set_layout_rect(ctx, data, env, child_size.to_rect());

        let self_size = bc.constrain(child_size);
        let _ = self.scroll_component.scroll(Vec2::new(0.0, 0.0), self_size);
        self.child.set_viewport_offset(self.offset());
        self_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.scroll_component
            .paint_content(ctx, env, |visible, ctx| {
                ctx.with_child_ctx(visible, |ctx| {
                    self.child.paint_raw(ctx, data, env)
                });
            });
    }
}
