use smithay_client_toolkit::reexports::client::{
    backend::ObjectId,
    protocol::{wl_subsurface::WlSubsurface, wl_surface::WlSurface},
    Dispatch, Proxy, QueueHandle,
};

use smithay_client_toolkit::{
    compositor::SurfaceData,
    subcompositor::{SubcompositorState, SubsurfaceData},
};

use crate::theme::{BORDER_SIZE, HEADER_SIZE, RESIZE_HANDLE_SIZE};
use crate::{pointer::Location, wl_typed::WlTyped};

/// The decoration's 'parts'.
#[derive(Debug)]
pub struct DecorationParts {
    pub surface: WlTyped<WlSurface, SurfaceData>,
    pub subsurface: WlTyped<WlSubsurface, SubsurfaceData>,
    parts: [Part; 5],
}

impl DecorationParts {
    // XXX keep in sync with `Self;:new`.
    pub const TOP: usize = 0;
    pub const LEFT: usize = 1;
    pub const RIGHT: usize = 2;
    pub const BOTTOM: usize = 3;
    pub const HEADER: usize = 4;

    pub fn new<State>(
        base_surface: &WlTyped<WlSurface, SurfaceData>,
        subcompositor: &SubcompositorState,
        queue_handle: &QueueHandle<State>,
    ) -> Self
    where
        State: Dispatch<WlSurface, SurfaceData> + Dispatch<WlSubsurface, SubsurfaceData> + 'static,
    {
        let (subsurface, surface) =
            subcompositor.create_subsurface(base_surface.inner().clone(), queue_handle);

        let subsurface = WlTyped::wrap::<State>(subsurface);
        let surface = WlTyped::wrap::<State>(surface);

        // Sync with the parent surface.
        subsurface.set_sync();

        // XXX the order must be in sync with associated constants.
        let parts = [
            // Top.
            Part {
                rect: Rect {
                    x: -(BORDER_SIZE as i32),
                    y: -(HEADER_SIZE as i32 + BORDER_SIZE as i32),
                    width: 0, // Defined by `Self::resize`.
                    height: BORDER_SIZE,
                },
                input_rect: Some(Rect {
                    x: BORDER_SIZE as i32 - RESIZE_HANDLE_SIZE as i32,
                    y: BORDER_SIZE as i32 - RESIZE_HANDLE_SIZE as i32,
                    width: 0, // Defined by `Self::resize`.
                    height: RESIZE_HANDLE_SIZE,
                }),
            },
            // Left.
            Part {
                rect: Rect {
                    x: -(BORDER_SIZE as i32),
                    y: -(HEADER_SIZE as i32),
                    width: BORDER_SIZE,
                    height: 0, // Defined by `Self::resize`.
                },
                input_rect: Some(Rect {
                    x: BORDER_SIZE as i32 - RESIZE_HANDLE_SIZE as i32,
                    y: 0,
                    width: RESIZE_HANDLE_SIZE,
                    height: 0, // Defined by `Self::resize`.
                }),
            },
            // Right.
            Part {
                rect: Rect {
                    x: 0, // Defined by `Self::resize`.
                    y: -(HEADER_SIZE as i32),
                    width: BORDER_SIZE,
                    height: 0, // Defined by `Self::resize`.
                },
                input_rect: Some(Rect {
                    x: 0,
                    y: 0,
                    width: RESIZE_HANDLE_SIZE,
                    height: 0, // Defined by `Self::resize`.
                }),
            },
            // Bottom.
            Part {
                rect: Rect {
                    x: -(BORDER_SIZE as i32),
                    y: 0,     // Defined by `Self::resize`.
                    width: 0, // Defined by `Self::resize`.
                    height: BORDER_SIZE,
                },
                input_rect: Some(Rect {
                    x: BORDER_SIZE as i32 - RESIZE_HANDLE_SIZE as i32,
                    y: 0,
                    width: 0, // Defined by `Self::resize`,
                    height: RESIZE_HANDLE_SIZE,
                }),
            },
            // Header.
            Part {
                rect: Rect {
                    x: 0,
                    y: -(HEADER_SIZE as i32),
                    width: 0, // Defined by `Self::resize`.
                    height: HEADER_SIZE,
                },
                input_rect: None,
            },
        ];

        Self {
            surface,
            subsurface,
            parts,
        }
    }

    pub fn parts(&self) -> std::iter::Enumerate<std::slice::Iter<Part>> {
        self.parts.iter().enumerate()
    }

    pub fn hide(&self) {
        self.subsurface.set_sync();
        self.surface.attach(None, 0, 0);
        self.surface.commit();
    }

    // These unwraps are guaranteed to succeed because the affected options are filled above
    // and then never emptied afterwards.
    #[allow(clippy::unwrap_used)]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.parts[Self::HEADER].rect.width = width;

        self.parts[Self::BOTTOM].rect.width = width + 2 * BORDER_SIZE;
        self.parts[Self::BOTTOM].rect.y = height as i32;
        self.parts[Self::BOTTOM].input_rect.as_mut().unwrap().width =
            self.parts[Self::BOTTOM].rect.width - (BORDER_SIZE * 2) + (RESIZE_HANDLE_SIZE * 2);

        self.parts[Self::TOP].rect.width = self.parts[Self::BOTTOM].rect.width;
        self.parts[Self::TOP].input_rect.as_mut().unwrap().width =
            self.parts[Self::TOP].rect.width - (BORDER_SIZE * 2) + (RESIZE_HANDLE_SIZE * 2);

        self.parts[Self::LEFT].rect.height = height + HEADER_SIZE;
        self.parts[Self::LEFT].input_rect.as_mut().unwrap().height =
            self.parts[Self::LEFT].rect.height;

        self.parts[Self::RIGHT].rect.height = self.parts[Self::LEFT].rect.height;
        self.parts[Self::RIGHT].rect.x = width as i32;
        self.parts[Self::RIGHT].input_rect.as_mut().unwrap().height =
            self.parts[Self::RIGHT].rect.height;
    }

    pub fn surface_rect(&self, draw_borders: bool) -> Rect {
        if draw_borders {
            Rect {
                height: self.parts[Self::TOP].rect.height
                    + self.parts[Self::LEFT].rect.height
                    + self.parts[Self::BOTTOM].rect.height,
                ..self.parts[Self::TOP].rect
            }
        } else {
            self.parts[Self::HEADER].rect
        }
    }

    pub fn header(&self) -> &Part {
        &self.parts[Self::HEADER]
    }

    pub fn side_height(&self) -> u32 {
        self.parts[Self::LEFT].rect.height
    }

    /// `x` and `y` are coordinates on our subsurface.
    pub fn find_part(&self, surface: &ObjectId, draw_borders: bool, x: f64, y: f64) -> Location {
        if surface != &self.surface.id() {
            return Location::None;
        }

        let surface_rect = self.surface_rect(draw_borders);
        // offset `x` and `y` to be relative to the main surface
        let x = x + surface_rect.x as f64;
        let y = y + surface_rect.y as f64;
        self.parts
            .iter()
            .position(|part| {
                let input_rect = part.input_rect();
                let rect_x = part.rect.x as f64 + input_rect.x as f64;
                let rect_y = part.rect.y as f64 + input_rect.y as f64;
                x >= rect_x
                    && x <= rect_x + input_rect.width as f64
                    && y >= rect_y
                    && y <= rect_y + input_rect.height as f64
            })
            .map(|pos| match pos {
                Self::HEADER => Location::Head,
                Self::TOP => Location::Top,
                Self::BOTTOM => Location::Bottom,
                Self::LEFT => Location::Left,
                Self::RIGHT => Location::Right,
                _ => unreachable!(),
            })
            .unwrap_or(Location::None)
    }
}

impl Drop for DecorationParts {
    fn drop(&mut self) {
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct Part {
    /// Positioned relative to the main surface.
    pub rect: Rect,
    /// Positioned relative to `rect`.
    ///
    /// `None` if it fully covers `rect`.
    pub input_rect: Option<Rect>,
}

impl Part {
    /// Positioned relative to `self.rect`.
    pub fn input_rect(&self) -> Rect {
        self.input_rect.unwrap_or(Rect {
            width: self.rect.width,
            height: self.rect.height,
            ..Default::default()
        })
    }
}
