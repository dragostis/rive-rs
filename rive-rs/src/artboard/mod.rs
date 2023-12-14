use alloc::sync::Arc;
use core::{
    fmt,
    marker::PhantomData,
    ptr::{self, NonNull},
    slice, str,
    time::Duration,
};

use crate::{
    ffi,
    file::{File, FileInner},
    instantiate::{Handle, Instantiate},
    linear_animation::Loop,
    renderer::Renderer,
    scene::{Scene, Viewport},
};

use self::components::Components;

pub mod components;

#[derive(Debug)]
pub(crate) struct ArtboardInner {
    _file: Arc<FileInner>,
    pub(crate) raw_artboard: *mut ffi::Artboard,
}

impl Drop for ArtboardInner {
    fn drop(&mut self) {
        unsafe {
            ffi::rive_rs_artboard_instance_release(self.raw_artboard);
        }
    }
}

unsafe impl Send for ArtboardInner {}
unsafe impl Sync for ArtboardInner {}

pub struct Artboard<R: Renderer> {
    inner: Arc<ArtboardInner>,
    _phantom: PhantomData<R>,
}

impl<R: Renderer> Artboard<R> {
    pub(crate) fn from_inner(inner: Arc<ArtboardInner>) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn as_inner(&self) -> &Arc<ArtboardInner> {
        &self.inner
    }

    #[inline]
    pub fn components(&mut self) -> Components {
        Components::new(components::RawArtboard(self.inner.raw_artboard))
    }
}

impl<R: Renderer> Instantiate for Artboard<R> {
    type From = File<R>;

    #[inline]
    fn instantiate(file: &Self::From, handle: Handle) -> Option<Self> {
        let mut raw_artboard: Option<NonNull<ffi::Artboard>> = None;

        match handle {
            Handle::Default => unsafe {
                ffi::rive_rs_instantiate_artboard(file.as_inner().raw_file, None, &mut raw_artboard)
            },
            Handle::Index(ref index) => unsafe {
                ffi::rive_rs_instantiate_artboard(
                    file.as_inner().raw_file,
                    Some(index.into()),
                    &mut raw_artboard,
                )
            },
            Handle::Name(name) => unsafe {
                ffi::rive_rs_instantiate_artboard_by_name(
                    file.as_inner().raw_file,
                    name.as_ptr(),
                    name.len(),
                    &mut raw_artboard,
                )
            },
        }

        raw_artboard.map(|raw_artboard| Artboard {
            inner: Arc::new(ArtboardInner {
                _file: file.as_inner().clone(),
                raw_artboard: raw_artboard.as_ptr(),
            }),
            _phantom: PhantomData,
        })
    }
}

impl<R: Renderer> fmt::Debug for Artboard<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ardboard").finish()
    }
}

unsafe impl<R: Renderer> Send for Artboard<R> {}
unsafe impl<R: Renderer> Sync for Artboard<R> {}

impl<R: Renderer> Scene<R> for Artboard<R> {
    fn width(&self) -> f32 {
        unsafe { ffi::rive_rs_artboard_width(self.inner.raw_artboard) }
    }

    fn height(&self) -> f32 {
        unsafe { ffi::rive_rs_artboard_height(self.inner.raw_artboard) }
    }

    fn name(&self) -> &str {
        let mut data = ptr::null();
        let mut len = 0;

        let bytes = unsafe {
            ffi::rive_rs_component_name(
                self.inner.raw_artboard as *const ffi::Component,
                &mut data as *mut *const u8,
                &mut len as *mut usize,
            );
            slice::from_raw_parts(data, len)
        };

        str::from_utf8(bytes).expect("component name is invalid UTF-8")
    }

    fn r#loop(&self) -> Loop {
        Loop::OneShot
    }

    fn is_translucent(&self) -> bool {
        false
    }

    fn duration(&self) -> Option<core::time::Duration> {
        None
    }

    fn pointer_down(&mut self, _x: f32, _y: f32, _viewport: &Viewport) {}

    fn pointer_move(&mut self, _x: f32, _y: f32, _viewport: &Viewport) {}

    fn pointer_up(&mut self, _x: f32, _y: f32, _viewport: &Viewport) {}

    fn advance_and_apply(&mut self, _elapsed: Duration) -> bool {
        unsafe {
            ffi::rive_rs_artboard_advance(self.inner.raw_artboard);
        }

        true
    }

    fn draw(&self, renderer: &mut R) {
        unsafe {
            ffi::rive_rs_artboard_draw(
                self.inner.raw_artboard,
                renderer as *mut R as *mut (),
                crate::ffi::RendererEntries::<R>::ENTRIES as *const crate::ffi::RendererEntries<R>
                    as *const (),
            );
        }
    }

    fn advance_and_maybe_draw(
        &mut self,
        renderer: &mut R,
        elapsed: Duration,
        viewport: &mut Viewport,
    ) -> bool {
        let mut view_transform = [0.0; 6];
        let mut inverse_view_transform = [0.0; 6];

        unsafe {
            crate::ffi::rive_rs_artboard_instance_transforms(
                self.inner.raw_artboard,
                viewport.width,
                viewport.height,
                view_transform.as_mut_ptr(),
                inverse_view_transform.as_mut_ptr(),
            );
        }

        viewport.inverse_view_transform = inverse_view_transform;

        if !self.advance_and_apply(elapsed) {
            // return false;
        }

        renderer.state_push();
        renderer.transform(&view_transform);

        self.draw(renderer);

        renderer.state_pop();

        true
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}
