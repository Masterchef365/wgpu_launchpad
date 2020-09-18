pub use wgpu;
pub use winit::event::WindowEvent;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

const SWAPCHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

/// Traits implemented by the rendered Scene 
pub trait Scene {
    /// Arguments passed to the type during launch
    type Args;

    /// Create a new instance of the scene; setup code should use the device to create pipelines
    fn new(device: &wgpu::Device, args: Self::Args) -> Self;

    /// Draw the scene; called every frame
    fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView);

    /// (Optional) handle events from Winit
    fn event(&mut self, _event: &WindowEvent) {}
}

/// Launch the scene. See `examples/triangle.rs`.
pub fn launch<S: 'static + Scene>(args: S::Args) {
    // Initialize winit
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    // Initialize wgpu
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };

    let (mut device, queue) = futures::executor::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Request adapter");

        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: false,
                },
                None,
            )
            .await
            .expect("Request device")
    });

    let mut swap_chain = {
        let size = window.inner_size();

        device.create_swap_chain(
            &surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: SWAPCHAIN_FORMAT,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Mailbox,
            },
        )
    };
    let mut resized = false;

    // Initialize scene and GUI controls
    let mut scene = S::new(&mut device, args);

    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event, .. } => {
                scene.event(&event);
                match event {
                    WindowEvent::Resized(_) => {
                        resized = true;
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            Event::MainEventsCleared => {
                // Rebuild the swapchain if necessary
                if resized {
                    let size = window.inner_size();

                    swap_chain = device.create_swap_chain(
                        &surface,
                        &wgpu::SwapChainDescriptor {
                            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                            format: SWAPCHAIN_FORMAT,
                            width: size.width,
                            height: size.height,
                            present_mode: wgpu::PresentMode::Mailbox,
                        },
                    );

                    resized = false;
                }

                // Get another frame
                let frame = swap_chain.get_current_frame().expect("Next frame");

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // Draw the scene
                scene.draw(&mut encoder, &frame.output.view);

                // Then we submit the work
                queue.submit(Some(encoder.finish()));
            }
            _ => {}
        }
    })
}
