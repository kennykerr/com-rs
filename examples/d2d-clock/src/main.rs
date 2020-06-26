use com::{com_interface, interfaces::IUnknown, ComInterface, ComPtr};
use winapi::shared::minwindef::FLOAT;
use winapi::um::winnt::HRESULT;

fn main() {
    let _ = AutoCoInitialize::new();
    ClockWindow::new().run()
}

// this should most likely just be a wrapper type
// much like we have in winrt
macro_rules! HR {
    ($hr:expr) => {
        if $hr != 0 {
            // S_OK
            panic!("non successful HRESULT 0x{:x}", $hr);
        }
    };
}
struct AutoCoInitialize;
impl AutoCoInitialize {
    fn new() -> Self {
        unsafe {
            HR!(winapi::um::combaseapi::CoInitializeEx(
                std::ptr::null_mut(),
                0x2
            ))
        };
        AutoCoInitialize
    }
}

impl Drop for AutoCoInitialize {
    fn drop(&mut self) {
        unsafe { winapi::um::combaseapi::CoUninitialize() };
    }
}

trait Window {
    fn run(&mut self);
}

struct ClockWindow<W> {
    window: W,
}

impl<W: Window> ClockWindow<W> {
    fn run(&mut self) {
        self.window.run();
    }
}

impl ClockWindow<DesktopWindow> {
    fn new() -> Self {
        Self {
            window: DesktopWindow::new(0.0),
        }
    }
}

struct DesktopWindow {
    dpix: f32,
}

impl DesktopWindow {
    fn new(dpix: f32) -> Self {
        Self { dpix }
    }
}

impl Window for DesktopWindow {
    fn run(&mut self) {
        let factory = create_factory();
        let mut dxgi_factory = std::ptr::null_mut();
        let _dxgi_factory = unsafe {
            HR!(winapi::shared::dxgi::CreateDXGIFactory1(
                &IDXGIFactory2::IID as *const _ as _,
                &mut dxgi_factory as *mut _,
            ));
            ComPtr::<dyn IDXGIFactory2>::new(dxgi_factory as *mut _)
        };
        let mut dpiy: f32 = 0.0;
        unsafe {
            factory.get_desktop_dpi(&mut self.dpix, &mut dpiy);
        }
        println!("DPI: {}x{}", self.dpix, dpiy);
        // TODO create device independent resources: CreateDeviceIndependentResources

        // TODO: VERIFY(__super::Create(nullptr, bounds, L"Direct2D"));

        // winapi::um::winuser::RegisterPowerSettingNotification
    }
}

fn create_factory() -> ComPtr<dyn ID2D1Factory1> {
    let fo = &winapi::um::d2d1::D2D1_FACTORY_OPTIONS::default();
    let mut factory = std::ptr::null_mut();
    unsafe {
        HR!(winapi::um::d2d1::D2D1CreateFactory(
            winapi::um::d2d1::D2D1_FACTORY_TYPE_SINGLE_THREADED,
            &ID2D1Factory1::IID as *const _ as _,
            fo as *const _,
            &mut factory,
        ));
        ComPtr::new(factory as _)
    }
}

#[com_interface("06152247-6f50-465a-9245-118bfd3b6007")]
pub trait ID2D1Factory: IUnknown {
    unsafe fn reload_system_metrics(&self) -> HRESULT;
    unsafe fn get_desktop_dpi(&self, dpi_x: *mut FLOAT, dpi_y: *mut FLOAT);
}

#[com_interface("bb12d362-daee-4b9a-aa1d-14ba401cfa1f")]
pub trait ID2D1Factory1: ID2D1Factory {}

#[com_interface("50c83a1c-e072-4c48-87b0-3630fa36a6d0")]
pub trait IDXGIFactory2: IUnknown {}
