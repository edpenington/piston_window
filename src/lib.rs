#![deny(missing_docs)]

//! The official Piston window back-end for the Piston game engine

extern crate piston;
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_graphics;
extern crate graphics;

use std::cell::RefCell;
use std::rc::Rc;
use std::any::Any;

use piston::{ event, window };
use gfx::traits::*;
use gfx_graphics::{ Gfx2d, GfxGraphics };
use graphics::Context;

/// Actual gfx::Stream implementation carried by the window.
pub type GfxStream = gfx::OwnedStream<gfx_device_gl::Device, gfx_device_gl::Output>;

/// Contains everything required for controlling window, graphics, event loop.
pub struct PistonWindow<W: window::Window, T = ()> {
    /// The window.
    pub window: Rc<RefCell<W>>,
    /// GFX stream.
    pub stream: Rc<RefCell<GfxStream>>,
    /// GFX device.
    pub device: Rc<RefCell<gfx_device_gl::Device>>,
    /// Gfx2d.
    pub g2d: Rc<RefCell<Gfx2d<gfx_device_gl::Resources>>>,
    /// The event loop.
    pub events: Rc<RefCell<event::WindowEvents<W, event::Event<W::Event>>>>,
    /// The event.
    pub event: Option<event::Event<W::Event>>,
    /// Application structure.
    pub app: Rc<RefCell<T>>,
    /// The factory that was created along with the device.
    pub factory: Rc<RefCell<gfx_device_gl::Factory>>,
}

impl<W, T> Clone for PistonWindow<W, T>
    where W: window::Window, W::Event: Clone
{
    fn clone(&self) -> Self {
        PistonWindow {
            window: self.window.clone(),
            stream: self.stream.clone(),
            device: self.device.clone(),
            g2d: self.g2d.clone(),
            events: self.events.clone(),
            event: self.event.clone(),
            app: self.app.clone(),
            factory: self.factory.clone(),
        }
    }
}

impl<W, T> PistonWindow<W, T>
    where W: window::Window, W::Event: event::GenericEvent
{
    /// Creates a new piston object.
    pub fn new(window: Rc<RefCell<W>>, app: Rc<RefCell<T>>) -> Self
        where W: window::OpenGLWindow
    {
        use piston::event::Events;
        use piston::window::{ OpenGLWindow, Window };

        let (device, mut factory) =
            gfx_device_gl::create(|s| window.borrow_mut().get_proc_address(s));

        let draw_size = window.borrow().draw_size();
        let output = factory.make_fake_output(draw_size.width as u16, draw_size.height as u16);

        let g2d = Gfx2d::new(&mut factory);

        let stream = factory.create_stream(output);

        PistonWindow {
            window: window.clone(),
            stream: Rc::new(RefCell::new(stream)),
            device: Rc::new(RefCell::new(device)),
            g2d: Rc::new(RefCell::new(g2d)),
            events: Rc::new(RefCell::new(window.events())),
            event: None,
            app: app,
            factory: Rc::new(RefCell::new(factory)),
        }
    }

    /// Changes application structure.
    pub fn app<U>(self, app: Rc<RefCell<U>>) -> PistonWindow<W, U> {
        PistonWindow {
            window: self.window,
            stream: self.stream,
            device: self.device,
            g2d: self.g2d,
            events: self.events,
            event: self.event,
            app: app,
            factory: self.factory,
        }
    }

    /// Renders 2D graphics.
    pub fn draw_2d<F>(&self, f: F) where
        F: FnMut(Context, &mut GfxGraphics<
            gfx_device_gl::Resources, gfx_device_gl::CommandBuffer,
            gfx_device_gl::Output>)
    {
        use piston::event::RenderEvent;

        if let Some(ref e) = self.event {
            if let Some(args) = e.render_args() {
                let mut stream = self.stream.borrow_mut();
                {
                    let (renderer, output) = stream.access();
                    self.g2d.borrow_mut().draw(renderer, output, args.viewport(), f);
                }
                stream.flush(&mut *self.device.borrow_mut());
            }
        }
    }

    /// Renders 3D graphics.
    pub fn draw_3d<F>(&self, mut f: F) where
        F: FnMut(&mut GfxStream)
    {
        use piston::event::RenderEvent;

        if let Some(ref e) = self.event {
            if let Some(_) = e.render_args() {
                let mut stream = self.stream.borrow_mut();
                f(&mut *stream);
                stream.flush(&mut *self.device.borrow_mut())
            }
        }
    }
}

impl<W, T> Iterator for PistonWindow<W, T>
    where W: window::Window, W::Event: event::GenericEvent
{
    type Item = PistonWindow<W, T>;

    fn next(&mut self) -> Option<PistonWindow<W, T>> {
        use piston::event::*;

        if let Some(e) = self.events.borrow_mut().next() {
            if let Some(_) = e.after_render_args() {
                // After swapping buffers.
                self.device.borrow_mut().cleanup();
            }

            if let Some(_) = e.resize_args() {
                let mut stream = self.stream.borrow_mut();
                let draw_size = self.window.borrow().draw_size();
                stream.out.width = draw_size.width as u16;
                stream.out.height = draw_size.height as u16;
            }

            Some(PistonWindow {
                window: self.window.clone(),
                stream: self.stream.clone(),
                device: self.device.clone(),
                g2d: self.g2d.clone(),
                events: self.events.clone(),
                event: Some(e),
                app: self.app.clone(),
                factory: self.factory.clone(),
            })
        } else { None }
    }
}

impl<W, T> event::GenericEvent for PistonWindow<W, T>
    where W: window::Window, W::Event: event::GenericEvent
{
    fn event_id(&self) -> event::EventId {
        match self.event {
            Some(ref e) => e.event_id(),
            None => event::EventId("")
        }
    }

    fn with_args<'a, F, U>(&'a self, f: F) -> U
       where F: FnMut(&Any) -> U
    {
        self.event.as_ref().unwrap().with_args(f)
    }

    fn from_args(event_id: event::EventId, any: &Any, old_event: &Self) -> Option<Self> {
        if let Some(ref e) = old_event.event {
            match event::GenericEvent::from_args(event_id, any, e) {
                Some(e) => {
                    Some(PistonWindow {
                        window: old_event.window.clone(),
                        stream: old_event.stream.clone(),
                        device: old_event.device.clone(),
                        g2d: old_event.g2d.clone(),
                        events: old_event.events.clone(),
                        event: Some(e),
                        app: old_event.app.clone(),
                        factory: old_event.factory.clone(),
                    })
                }
                None => None
            }
        } else { None }
    }
}

impl<W, T> window::Window for PistonWindow<W, T>
    where W: window::Window
{
    type Event = <W as window::Window>::Event;

    fn should_close(&self) -> bool { self.window.borrow().should_close() }
    fn size(&self) -> window::Size { self.window.borrow().size() }
    fn draw_size(&self) -> window::Size { self.window.borrow().draw_size() }
    fn swap_buffers(&mut self) { self.window.borrow_mut().swap_buffers() }
    fn poll_event(&mut self) -> Option<Self::Event> {
        window::Window::poll_event(&mut *self.window.borrow_mut())
    }
}

impl<W, T> window::AdvancedWindow for PistonWindow<W, T>
    where W: window::AdvancedWindow
{
    fn get_title(&self) -> String { self.window.borrow().get_title() }
    fn set_title(&mut self, title: String) {
        self.window.borrow_mut().set_title(title)
    }
    fn get_exit_on_esc(&self) -> bool { self.window.borrow().get_exit_on_esc() }
    fn set_exit_on_esc(&mut self, value: bool) {
        self.window.borrow_mut().set_exit_on_esc(value)
    }
    fn set_capture_cursor(&mut self, value: bool) {
        self.window.borrow_mut().set_capture_cursor(value)
    }
}

/// Creates a new empty application.
pub fn empty_app() -> Rc<RefCell<()>> { Rc::new(RefCell::new(())) }
