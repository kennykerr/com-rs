use com::{com_interface, interfaces::IUnknown, ComInterface, ComPtr};
use winapi::shared::minwindef::FLOAT;
use winapi::um::winnt::HRESULT;

fn main() {
    com::runtime::init_apartment(com::runtime::ApartmentType::SingleThreaded).unwrap();
    ClockWindow::new().run();
    com::runtime::deinit_apartment();
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

#[repr(C)]
struct DesktopWindow {
    dpix: f32,
    window: Option<winapi::shared::windef::HWND>,
}

// extern "C" {
//     __ImageBase: winapi::um::winnt::IMAGE_DOS_HEADER ;
// }

impl DesktopWindow {
    fn new(dpix: f32) -> Self {
        // WNDCLASS wc{};
        let mut wc = winapi::um::winuser::WNDCLASSEXA::default();
        unsafe {
            wc.hCursor = winapi::um::winuser::LoadCursorW(
                std::ptr::null_mut(),
                winapi::um::winuser::IDC_ARROW,
            );
            // wc.hInstance = reinterpret_cast<HINSTANCE>(&__ImageBase);
            wc.lpszClassName = b"Sample" as *const u8 as _;
            wc.style = winapi::um::winuser::CS_HREDRAW | winapi::um::winuser::CS_VREDRAW;
            wc.lpfnWndProc = Some(window_proc);
        }
        // RegisterClass(&wc);

        // CreateWindow(wc.lpszClassName,
        //     L"Clock",
        //     WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        //     CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
        //     nullptr, nullptr, wc.hInstance, this);
        Self { dpix, window: None }
    }
    unsafe fn message_handler(
        &self,
        message: u32,
        wparam: winapi::shared::minwindef::WPARAM,
        lparam: winapi::shared::minwindef::LPARAM,
    ) -> winapi::shared::minwindef::LRESULT {
        match message {
            winapi::um::winuser::WM_DESTROY => {
                winapi::um::winuser::PostQuitMessage(0);
                0
            }
            winapi::um::winuser::WM_PAINT => {
                // PAINTSTRUCT ps;
                // check_bool(BeginPaint(m_window, &ps));
                // render();
                // EndPaint(m_window, &ps);
                0
            }
            winapi::um::winuser::WM_SIZE => {
                // if m_target && SIZE_MINIMIZED != wparam
                // {
                //     resize_swapchain_bitmap();
                //     render();
                // }

                0
            }
            winapi::um::winuser::WM_DISPLAYCHANGE => {
                // render();
                0
            }
            winapi::um::winuser::WM_USER => {
                // if (S_OK == m_swapChain->Present(0, DXGI_PRESENT_TEST))
                // {
                //     m_dxfactory->UnregisterOcclusionStatus(m_occlusion);
                //     m_occlusion = 0;
                //     m_visible = true;
                // }

                0
            }
            winapi::um::winuser::WM_POWERBROADCAST => {
                // auto const ps = reinterpret_cast<POWERBROADCAST_SETTING*>(lparam);
                // m_visible = 0 != *reinterpret_cast<DWORD const*>(ps->Data);

                // if (m_visible)
                // {
                //     PostMessage(m_window, WM_NULL, 0, 0);
                // }

                winapi::shared::minwindef::TRUE as isize
            }
            winapi::um::winuser::WM_ACTIVATE => {
                // m_visible = !HIWORD(wparam);
                0
            }
            winapi::um::winuser::WM_GETMINMAXINFO => {
                // auto info = reinterpret_cast<MINMAXINFO*>(lparam);
                // info->ptMinTrackSize.y = 200;

                0
            }
            _ => winapi::um::winuser::DefWindowProcW(self.window(), message, wparam, lparam),
        }
    }

    fn window(&self) -> winapi::shared::windef::HWND {
        self.window.expect("Tried to use window before it was set")
    }
}

unsafe extern "system" fn window_proc(
    window: winapi::shared::windef::HWND,
    message: u32,
    wparam: winapi::shared::minwindef::WPARAM,
    lparam: winapi::shared::minwindef::LPARAM,
) -> winapi::shared::minwindef::LRESULT {
    if winapi::um::winuser::WM_NCCREATE == message {
        let cs = lparam as *mut winapi::um::winuser::CREATESTRUCTW;
        let that = (*cs).lpCreateParams as *mut DesktopWindow;
        (*that).window = Some(window);
        winapi::um::winuser::SetWindowLongPtrW(
            window,
            winapi::um::winuser::GWLP_USERDATA,
            that as isize,
        );
    } else {
        let that =
            winapi::um::winuser::GetWindowLongPtrW(window, winapi::um::winuser::GWLP_USERDATA);
        let that = that as usize as *const DesktopWindow;
        if !that.is_null() {
            return (*that).message_handler(message, wparam, lparam);
        }
    }

    winapi::um::winuser::DefWindowProcW(window, message, wparam, lparam)
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
        // TODO create device independent resources: create_device_independent_resources

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
