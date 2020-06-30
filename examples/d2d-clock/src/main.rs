use com::{com_interface, interfaces::IUnknown, ComInterface, ComPtr, ComRc};
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
    ($hr:expr) => {{
        let hr = $hr;
        if hr != 0 {
            panic!("non successful HRESULT 0x{:x}", hr);
        }
    }};
}

macro_rules! check_bool {
    ($bool:expr) => {
        if !$bool.to_bool() {
            panic!("non successful action: {}", stringify!($bool));
        }
    };
}

trait BoolLike {
    fn to_bool(self) -> bool;
}
impl<T> BoolLike for *mut T {
    fn to_bool(self) -> bool {
        !self.is_null()
    }
}
impl<T> BoolLike for *const T {
    fn to_bool(self) -> bool {
        !self.is_null()
    }
}
macro_rules! primitive_bool {
    ($($t:ty),*) => {
        $(
            impl BoolLike for $t {
                fn to_bool(self) -> bool {
                    self == 0
                }
            }
        )*
    };
}
primitive_bool!(u16, i32);

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
    visible: bool,
    target: Option<ComRc<dyn ID2D1DeviceContext>>,
    factory: Option<ComRc<dyn ID2D1Factory1>>,
}

// extern "C" {
//     __ImageBase: winapi::um::winnt::IMAGE_DOS_HEADER ;
// }

impl DesktopWindow {
    fn new(dpix: f32) -> Self {
        // WNDCLASS wc{};
        let mut wc = winapi::um::winuser::WNDCLASSW::default();
        unsafe {
            wc.hCursor = winapi::um::winuser::LoadCursorW(
                std::ptr::null_mut(),
                winapi::um::winuser::IDC_ARROW,
            );
            // wc.hInstance = reinterpret_cast<HINSTANCE>(&__ImageBase);
            // wc.lpszClassName = L"Sample";
            wc.style = winapi::um::winuser::CS_HREDRAW | winapi::um::winuser::CS_VREDRAW;
            wc.lpfnWndProc = Some(window_proc);
            winapi::um::winuser::RegisterClassW(&wc as *const _);
            // winapi::um::winuser::CreateWindowExW(
            //     wc.lpszClassName,
            //     [0],
            //     winapi::um::winuser::WS_OVERLAPPEDWINDOW | winapi::um::winuser::WS_VISIBLE,
            //     winapi::um::winuser::CW_USEDEFAULT,
            //     winapi::um::winuser::CW_USEDEFAULT,
            //     winapi::um::winuser::CW_USEDEFAULT,
            //     winapi::um::winuser::CW_USEDEFAULT,
            //     std::ptr::null_mut(),
            //     std::ptr::null_mut(),
            //     wc.hInstance,
            //     self,
            //     0,
            // );
        }

        Self {
            dpix,
            window: None,
            visible: false,
            target: None,
            factory: None,
        }
    }

    unsafe fn message_handler(
        &mut self,
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
                let ps = &mut winapi::um::winuser::PAINTSTRUCT::default();
                check_bool!(winapi::um::winuser::BeginPaint(self.window(), ps as *mut _));
                self.render();
                check_bool!(!winapi::um::winuser::EndPaint(self.window(), ps as *mut _));
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
                self.render();
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
                let ps = lparam as *const winapi::um::winuser::POWERBROADCAST_SETTING;
                self.visible = (*ps).Data != [0];

                if self.visible {
                    winapi::um::winuser::PostMessageW(
                        self.window(),
                        winapi::um::winuser::WM_NULL,
                        0,
                        0,
                    );
                }

                winapi::shared::minwindef::TRUE as isize
            }
            winapi::um::winuser::WM_ACTIVATE => {
                self.visible = !winapi::shared::minwindef::HIWORD(wparam as u32).to_bool();
                0
            }
            winapi::um::winuser::WM_GETMINMAXINFO => {
                let info = lparam as *mut winapi::um::winuser::MINMAXINFO;
                (*info).ptMinTrackSize.y = 200;
                0
            }
            _ => winapi::um::winuser::DefWindowProcW(self.window(), message, wparam, lparam),
        }
    }

    fn render(&mut self) {
        if self.target.is_none() {
            let mut device = create_device();
            self.target = Some(create_render_target(
                self.factory.as_ref().unwrap(),
                &mut device,
            ));
            //     m_swapChain = create_swapchain(device, m_window);
            //     create_swapchain_bitmap(m_swapChain, m_target);

            //     m_target->SetDpi(m_dpi, m_dpi);

            //     create_device_resources();
            //     create_device_size_resources();
        }

        // m_target->BeginDraw();
        // draw();
        // m_target->EndDraw();

        // auto const hr = m_swapChain->Present(1, 0);

        // if (S_OK == hr)
        // {
        //     // Do nothing
        // }
        // else if (DXGI_STATUS_OCCLUDED == hr)
        // {
        //     check_hresult(m_dxfactory->RegisterOcclusionStatusWindow(m_window, WM_USER, &m_occlusion));
        //     m_visible = false;
        // }
        // else
        // {
        //     release_device();
        // }
    }

    fn window(&self) -> winapi::shared::windef::HWND {
        self.window.expect("Tried to use window before it was set")
    }
}

fn create_render_target(
    factory: &ComRc<dyn ID2D1Factory1>,
    device: &mut ComRc<dyn ID3D11Device>,
) -> ComRc<dyn ID2D1DeviceContext> {
    let dxdevice = device.get_interface::<dyn IDXGIDevice>();

    let mut d2device: Option<ComPtr<dyn ID2D1Device>> = None;
    let target = unsafe {
        HR!(factory.create_device(dxdevice.map(|c| c.into()), &mut d2device as *mut _));
        let mut target: Option<ComPtr<dyn ID2D1DeviceContext>> = None;

        HR!(d2device.unwrap().create_device_context(
            winapi::um::d2d1_1::D2D1_DEVICE_CONTEXT_OPTIONS_NONE,
            &mut target as *mut _
        ));
        target
    };

    ComRc::new(target.unwrap())
}

fn create_device() -> ComRc<dyn ID3D11Device> {
    fn create_device(
        typ: winapi::um::d3dcommon::D3D_DRIVER_TYPE,
        device: &mut Option<ComRc<dyn ID3D11Device>>,
    ) -> HRESULT {
        let flags = winapi::um::d3d11::D3D11_CREATE_DEVICE_BGRA_SUPPORT;

        // #ifdef _DEBUG
        //     flags |= D3D11_CREATE_DEVICE_DEBUG;
        // #endif

        unsafe {
            winapi::um::d3d11::D3D11CreateDevice(
                std::ptr::null_mut(),
                typ,
                std::ptr::null_mut(),
                flags,
                std::ptr::null_mut(),
                0,
                winapi::um::d3d11::D3D11_SDK_VERSION,
                device as *const _ as *mut _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    }
    let mut device = None;
    let mut hr = create_device(winapi::um::d3dcommon::D3D_DRIVER_TYPE_HARDWARE, &mut device);

    if winapi::shared::winerror::DXGI_ERROR_UNSUPPORTED == hr {
        hr = create_device(winapi::um::d3dcommon::D3D_DRIVER_TYPE_WARP, &mut device);
    }

    HR!(hr);
    device.unwrap()
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
        let that = that as usize as *mut DesktopWindow;
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
            ComRc::<dyn IDXGIFactory2>::from_raw(dxgi_factory as *mut _)
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
    unsafe fn f2(&self);
    unsafe fn f3(&self);
    unsafe fn f4(&self);
    unsafe fn f5(&self);
    unsafe fn f6(&self);
    unsafe fn f7(&self);
    unsafe fn f8(&self);
    unsafe fn f9(&self);
    unsafe fn f10(&self);
    unsafe fn f11(&self);
    unsafe fn f12(&self);
    unsafe fn f13(&self);
}

#[com_interface("bb12d362-daee-4b9a-aa1d-14ba401cfa1f")]
pub trait ID2D1Factory1: ID2D1Factory {
    unsafe fn create_device(
        &self,
        dxgi_device: Option<ComPtr<dyn IDXGIDevice>>,
        d2d_device: *mut Option<ComPtr<dyn ID2D1Device>>,
    ) -> HRESULT;
}

#[com_interface("50c83a1c-e072-4c48-87b0-3630fa36a6d0")]
pub trait IDXGIFactory2: IUnknown {}

#[com_interface("e8f7fe7a-191c-466d-ad95-975678bda998")]
pub trait ID2D1DeviceContext: ID2D1RenderTarget {}

#[com_interface("47dd575d-ac05-4cdd-8049-9b02cd16f44c")]
pub trait ID2D1Device: ID2D1Resource {
    unsafe fn create_device_context(
        &self,
        options: winapi::um::d2d1_1::D2D1_DEVICE_CONTEXT_OPTIONS,
        device_context: *mut Option<ComPtr<dyn ID2D1DeviceContext>>,
    ) -> HRESULT;
}

#[com_interface("2cd90694-12e2-11dc-9fed-001143a055f9")]
pub trait ID2D1RenderTarget: ID2D1Resource {}

#[com_interface("2cd90691-12e2-11dc-9fed-001143a055f9")]
pub trait ID2D1Resource: IUnknown {
    unsafe fn r1(&self);
}

#[com_interface("db6f6ddb-ac77-4e88-8253-819df9bbf140")]
pub trait ID3D11Device: IUnknown {}

#[com_interface("54ec77fa-1377-44e6-8c32-88fd5f44c84c")]
pub trait IDXGIDevice: IDXGIObject {
    unsafe fn d0(&self);
    unsafe fn d1(&self);
    unsafe fn d2(&self);
    unsafe fn d3(&self);
    unsafe fn d4(&self);
}

#[com_interface("aec22fb8-76f3-4639-9be0-28eb43a67a2e")]
pub trait IDXGIObject: IUnknown {
    unsafe fn o0(&self);
    unsafe fn o1(&self);
    unsafe fn o2(&self);
    unsafe fn o3(&self);
}
