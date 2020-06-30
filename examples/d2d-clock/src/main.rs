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
            #[allow(unused_unsafe)]
            let error = unsafe { winapi::um::errhandlingapi::GetLastError() };
            panic!("non successful action: {} - {:x}", stringify!($bool), error);
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
                    self != 0
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
    window: winapi::shared::windef::HWND,
    visible: bool,
    // orientation: winapi::um::d2d1::D2D1_MATRIX_3X2_F,
    frequency: winapi::shared::ntdef::LARGE_INTEGER,
    target: Option<ComRc<dyn ID2D1DeviceContext>>,
    factory: Option<ComRc<dyn ID2D1Factory1>>,
    swap_chain: Option<ComRc<dyn IDXGISwapChain1>>,
    manager: Option<ComRc<dyn IUIAnimationManager>>,
    clock: Option<ComRc<dyn ID2D1Bitmap1>>,
    style: Option<ComRc<dyn ID2D1StrokeStyle>>,
}

// extern "C" {
//     __ImageBase: winapi::um::winnt::IMAGE_DOS_HEADER ;
// }

impl DesktopWindow {
    fn new(dpix: f32) -> Self {
        let mut wc = winapi::um::winuser::WNDCLASSW::default();

        let mut this = Self {
            dpix,
            window: std::ptr::null_mut(),
            visible: false,
            target: None,
            factory: None,
            swap_chain: None,
            frequency: winapi::shared::ntdef::LARGE_INTEGER::default(),
            // orientation: todo!(),
            manager: None,
            clock: None,
            style: None,
        };
        unsafe {
            wc.hCursor = winapi::um::winuser::LoadCursorW(
                std::ptr::null_mut(),
                winapi::um::winuser::IDC_ARROW,
            );
            wc.hInstance = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null_mut());
            let name = [b'h' as u16, 0u16];
            wc.lpszClassName = &name as *const _;
            wc.style = winapi::um::winuser::CS_HREDRAW | winapi::um::winuser::CS_VREDRAW;
            wc.lpfnWndProc = Some(window_proc);
            println!("Registering...");

            check_bool!(winapi::um::winuser::RegisterClassW(&wc as *const _));
            let name = [b'h' as u16, 0u16];
            winapi::um::winuser::CreateWindowExW(
                0,
                wc.lpszClassName,
                &name as *const u16,
                winapi::um::winuser::WS_OVERLAPPEDWINDOW | winapi::um::winuser::WS_VISIBLE,
                winapi::um::winuser::CW_USEDEFAULT,
                winapi::um::winuser::CW_USEDEFAULT,
                winapi::um::winuser::CW_USEDEFAULT,
                winapi::um::winuser::CW_USEDEFAULT,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                wc.hInstance,
                &mut this as *mut _ as _,
            );
            println!("Window created!");
        }

        this
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
                check_bool!(winapi::um::winuser::BeginPaint(self.window, ps as *mut _));
                self.render();
                check_bool!(!winapi::um::winuser::EndPaint(self.window, ps as *mut _));
                0
            }
            winapi::um::winuser::WM_SIZE => {
                if self.target.is_some() && winapi::um::winuser::SIZE_MINIMIZED != wparam {
                    // resize_swapchain_bitmap();
                    self.render();
                }

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
                        self.window,
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
            _ => winapi::um::winuser::DefWindowProcW(self.window, message, wparam, lparam),
        }
    }

    fn render(&mut self) {
        let (target, swap_chain) = match self.target {
            None => {
                let mut device = create_device();
                let target = create_render_target(self.factory.as_ref().unwrap(), &mut device);
                self.target = Some(target.clone());
                let swap_chain = create_swapchain(&device, self.window);
                self.swap_chain = Some(swap_chain.clone());

                create_swapchain_bitmap(&swap_chain, &target);

                unsafe { target.set_dpi(self.dpix, self.dpix) };

                //     create_device_resources();
                //     create_device_size_resources();
                (target, swap_chain)
            }
            Some(ref t) => (t.clone(), self.swap_chain.as_ref().unwrap().clone()),
        };

        unsafe { target.begin_draw() };
        self.draw();
        let hr = unsafe {
            target.end_draw(std::ptr::null_mut(), std::ptr::null_mut());
            swap_chain.present(1, 0)
        };

        match hr {
            winapi::shared::winerror::S_OK => {}
            winapi::shared::winerror::DXGI_STATUS_OCCLUDED => {
                // HR!(self.dx_factory.register_occlusion_status_window(
                //     self.window(),
                //     winapi::um::winuser::WM_USER,
                //     &self.occlusion
                // ));
                self.visible = false;
            }
            _ => {
                //     release_device();
            }
        };
    }

    fn draw(&mut self) {
        // TODO: identity
        // self.orientation = winapi::um::dcommon::D2D_MATRIX_3X2_F::default();
        // let offset = SizeF(5.0, 5.0);
        unsafe {
            HR!(self.manager.as_ref().unwrap().update(self.get_time()));
            let target = self.target.clone().unwrap();
            target.set_unit_mode(winapi::um::d2d1_1::D2D1_UNIT_MODE_PIXELS);
            let color_white = winapi::um::d2d1::D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };
            target.clear(&color_white);
            target.set_unit_mode(winapi::um::d2d1_1::D2D1_UNIT_MODE_DIPS);
            let mut previous: Option<ComPtr<dyn ID2D1Image>> = None;
            target.get_target(&mut previous);
            let clock = self.clock.clone().unwrap();
            let clock = ComPtr::new(clock.as_raw() as _);
            target.set_target(clock.clone());
            target.clear(std::ptr::null_mut());
            self.draw_clock();
            target.set_target(previous.unwrap());
            // target.set_transform(Matrix3x2F::Translation(offset));

            // target.draw_image(
            //     self.shadow.get(),
            //     D2D1_INTERPOLATION_MODE_LINEAR,
            //     D2D1_COMPOSITE_MODE_SOURCE_OVER,
            // );

            // m_target->SetTransform(Matrix3x2F::Identity());

            target.draw_image(
                clock,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                winapi::um::d2d1_1::D2D1_INTERPOLATION_MODE::default(),
                winapi::um::d2d1_1::D2D1_COMPOSITE_MODE::default(),
            );
        }
    }

    fn draw_clock(&mut self) {}

    fn get_time(&self) -> f64 {
        let mut time = winapi::shared::ntdef::LARGE_INTEGER::default();
        unsafe {
            check_bool!(winapi::um::profileapi::QueryPerformanceCounter(&mut time));
            *time.QuadPart() as f64 / *self.frequency.QuadPart() as f64
        }
    }

    fn create_device_independent_resources(&mut self) {
        let mut style = winapi::um::d2d1_1::D2D1_STROKE_STYLE_PROPERTIES1::default();
        style.startCap = winapi::um::d2d1::D2D1_CAP_STYLE_ROUND;
        style.endCap = winapi::um::d2d1::D2D1_CAP_STYLE_TRIANGLE;

        unsafe {
            let mut style_obj = ComPtr::new(self.style.as_ref().unwrap().as_raw() as _);
            HR!(self.factory.as_ref().unwrap().create_stroke_style(
                &style,
                std::ptr::null_mut(),
                0,
                &mut style_obj
            ));
        }

        self.schedule_animation();
    }

    fn schedule_animation(&mut self) {}
}

fn create_swapchain_bitmap(
    swap_chain: &ComRc<dyn IDXGISwapChain1>,
    target: &ComRc<dyn ID2D1DeviceContext>,
) {
    let mut ptr = std::ptr::null_mut();
    unsafe {
        HR!(swap_chain.get_buffer(0, &IDXGISurface::IID as *const _ as _, &mut ptr,));
        let surface: ComPtr<dyn IDXGISurface> = ComPtr::new(ptr as _);

        let mut props = winapi::um::d2d1_1::D2D1_BITMAP_PROPERTIES1::default();
        props.pixelFormat = winapi::um::dcommon::D2D1_PIXEL_FORMAT {
            format: winapi::shared::dxgiformat::DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: winapi::um::dcommon::D2D1_ALPHA_MODE_IGNORE,
        };
        props.bitmapOptions = winapi::um::d2d1_1::D2D1_BITMAP_OPTIONS_TARGET
            | winapi::um::d2d1_1::D2D1_BITMAP_OPTIONS_CANNOT_DRAW;

        let mut bitmap: Option<ComPtr<dyn ID2D1Bitmap1>> = None;

        HR!(target.create_bitmap_from_dxgi_surface(surface, &props, &mut bitmap));
        let bitmap = ComPtr::new(bitmap.unwrap().as_raw() as _);
        target.set_target(bitmap);
    }
}

extern "system" {}

fn create_swapchain(
    device: &ComRc<dyn ID3D11Device>,
    window: winapi::shared::windef::HWND,
) -> ComRc<dyn IDXGISwapChain1> {
    let factory = get_dxgi_factory(device);

    let mut props = winapi::shared::dxgi1_2::DXGI_SWAP_CHAIN_DESC1::default();
    props.Format = winapi::shared::dxgiformat::DXGI_FORMAT_B8G8R8A8_UNORM;
    props.SampleDesc.Count = 1;
    props.BufferUsage = winapi::shared::dxgitype::DXGI_USAGE_RENDER_TARGET_OUTPUT;
    props.BufferCount = 2;
    props.SwapEffect = winapi::shared::dxgi::DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL;

    let mut swap_chain: Option<ComPtr<dyn IDXGISwapChain1>> = None;

    unsafe {
        let device =
            ComPtr::new(device.as_raw() as *mut *mut <dyn IUnknown as ComInterface>::VTable);
        HR!(factory.create_swap_chain_for_hwnd(
            device,
            window,
            &props,
            std::ptr::null_mut(),
            None,
            &mut swap_chain
        ))
    };

    swap_chain.unwrap().upgrade()
}

fn get_dxgi_factory(device: &ComRc<dyn ID3D11Device>) -> ComRc<dyn IDXGIFactory2> {
    let dxdevice = device.get_interface::<dyn IDXGIDevice>().unwrap();
    let mut adapter: Option<ComPtr<dyn IDXGIAdapter>> = None;
    unsafe {
        HR!(dxdevice.get_adapter(&mut adapter as *mut _));
        let mut ptr = std::ptr::null_mut();
        HR!(adapter
            .unwrap()
            .get_parent(&IDXGIFactory2::IID as *const _ as _, &mut ptr as *mut _));
        ComRc::from_raw(ptr as *mut _)
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
        (*that).window = window;
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
        let factory = create_factory().upgrade();
        self.factory = Some(factory.clone());
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
            self.create_device_independent_resources();

            check_bool!(winapi::um::winuser::RegisterPowerSettingNotification(
                self.window as _,
                &winapi::um::winnt::GUID_SESSION_DISPLAY_STATUS,
                winapi::um::winuser::DEVICE_NOTIFY_WINDOW_HANDLE,
            ))
        }
        let mut message = winapi::um::winuser::MSG::default();
        loop {
            if self.visible {
                self.render();

                unsafe {
                    while winapi::um::winuser::PeekMessageW(
                        &mut message,
                        std::ptr::null_mut(),
                        0,
                        0,
                        winapi::um::winuser::PM_REMOVE,
                    )
                    .to_bool()
                    {
                        winapi::um::winuser::DispatchMessageW(&message);
                    }
                }
            } else {
                unsafe {
                    let result =
                        winapi::um::winuser::GetMessageW(&mut message, std::ptr::null_mut(), 0, 0);
                    if result.to_bool() {
                        if result != -1 {
                            winapi::um::winuser::DispatchMessageW(&message);
                        }
                    }
                }
            }

            if winapi::um::winuser::WM_QUIT == message.message {
                break;
            }
        }
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
    unsafe fn create_stroke_style(
        &self,
        strokeStyleProperties: *const winapi::um::d2d1_1::D2D1_STROKE_STYLE_PROPERTIES1,
        dashes: *const FLOAT,
        dashesCount: winapi::shared::basetsd::UINT32,
        strokeStyle: *mut ComPtr<dyn ID2D1StrokeStyle1>,
    ) -> HRESULT;
}

#[com_interface("50c83a1c-e072-4c48-87b0-3630fa36a6d0")]
pub trait IDXGIFactory2: IDXGIFactory1 {
    unsafe fn gif0(&self);
    unsafe fn create_swap_chain_for_hwnd(
        &self,
        p_device: ComPtr<dyn IUnknown>,
        hwnd: winapi::shared::windef::HWND,
        p_desc: *const winapi::shared::dxgi1_2::DXGI_SWAP_CHAIN_DESC1,
        p_fullscreen_desc: *const winapi::shared::dxgi1_2::DXGI_SWAP_CHAIN_FULLSCREEN_DESC,
        p_restrict_to_output: Option<ComPtr<dyn IDXGIOutput>>,
        pp_swapchain: *mut Option<ComPtr<dyn IDXGISwapChain1>>,
    ) -> HRESULT;
}

#[com_interface("770aae78-f26f-4dba-a829-253c83d1b387")]
pub trait IDXGIFactory1: IDXGIFactory {
    unsafe fn f10(&self);
    unsafe fn f11(&self);
}

#[com_interface("7b7166ec-21c7-44ae-b21a-c9ae321ae369")]
pub trait IDXGIFactory: IDXGIObject {
    unsafe fn f0(&self);
    unsafe fn f1(&self);
    unsafe fn f2(&self);
    unsafe fn f3(&self);
    unsafe fn f4(&self);
}

#[com_interface("e8f7fe7a-191c-466d-ad95-975678bda998")]
pub trait ID2D1DeviceContext: ID2D1RenderTarget {
    unsafe fn createbitmap(&self);
    unsafe fn createbitmapfromwicbitmap(&self);
    unsafe fn createcolorcontext(&self);
    unsafe fn createcolorcontextfromfilename(&self);
    unsafe fn createcolorcontextfromwiccolorcontext(&self);
    unsafe fn create_bitmap_from_dxgi_surface(
        &self,
        surface: ComPtr<dyn IDXGISurface>,
        bitmap_properties: *const winapi::um::d2d1_1::D2D1_BITMAP_PROPERTIES1,
        bitmap: *mut Option<ComPtr<dyn ID2D1Bitmap1>>,
    ) -> HRESULT;
    unsafe fn createeffect(&self);
    unsafe fn creategradientstopcollection(&self);
    unsafe fn createimagebrush(&self);
    unsafe fn createbitmapbrush(&self);
    unsafe fn createcommandlist(&self);
    unsafe fn isdxgiformatsupported(&self);
    unsafe fn isbufferprecisionsupported(&self);
    unsafe fn getimagelocalbounds(&self);
    unsafe fn getimageworldbounds(&self);
    unsafe fn getglyphrunworldbounds(&self);
    unsafe fn getdevice(&self);
    unsafe fn set_target(&self, image: ComPtr<dyn ID2D1Image>);
    unsafe fn get_target(&self, image: *mut Option<ComPtr<dyn ID2D1Image>>);
    unsafe fn setrenderingcontrols(&self);
    unsafe fn getrenderingcontrols(&self);
    unsafe fn setprimitiveblend(&self);
    unsafe fn getprimitiveblend(&self);
    unsafe fn set_unit_mode(&self, unit_mode: winapi::um::d2d1_1::D2D1_UNIT_MODE);
    unsafe fn getunitmode(&self);
    unsafe fn drawglyphrun(&self);
    unsafe fn draw_image(
        &self,
        image: ComPtr<dyn ID2D1Image>,
        target_offset: *const winapi::um::d2d1::D2D1_POINT_2F,
        image_rectangle: *const winapi::um::d2d1::D2D1_RECT_F,
        interpolation_mode: winapi::um::d2d1_1::D2D1_INTERPOLATION_MODE,
        composite_mode: winapi::um::d2d1_1::D2D1_COMPOSITE_MODE,
    );
}

#[com_interface("47dd575d-ac05-4cdd-8049-9b02cd16f44c")]
pub trait ID2D1Device: ID2D1Resource {
    unsafe fn create_device_context(
        &self,
        options: winapi::um::d2d1_1::D2D1_DEVICE_CONTEXT_OPTIONS,
        device_context: *mut Option<ComPtr<dyn ID2D1DeviceContext>>,
    ) -> HRESULT;
}

#[com_interface("2cd90694-12e2-11dc-9fed-001143a055f9")]
pub trait ID2D1RenderTarget: ID2D1Resource {
    unsafe fn rt0(&self);
    unsafe fn rt1(&self);
    unsafe fn rt2(&self);
    unsafe fn rt3(&self);
    unsafe fn rt4(&self);
    unsafe fn rt5(&self);
    unsafe fn rt6(&self);
    unsafe fn rt7(&self);
    unsafe fn rt8(&self);
    unsafe fn rt9(&self);
    unsafe fn rt10(&self);
    unsafe fn rt11(&self);
    unsafe fn rt12(&self);
    unsafe fn rt13(&self);
    unsafe fn rt14(&self);
    unsafe fn rt15(&self);
    unsafe fn rt16(&self);
    unsafe fn rt17(&self);
    unsafe fn rt18(&self);
    unsafe fn rt19(&self);
    unsafe fn rt20(&self);
    unsafe fn rt21(&self);
    unsafe fn rt22(&self);
    unsafe fn rt23(&self);
    unsafe fn rt24(&self);
    unsafe fn rt25(&self);
    unsafe fn rt26(&self);
    unsafe fn rt27(&self);
    unsafe fn rt28(&self);
    unsafe fn rt29(&self);
    unsafe fn rt30(&self);
    unsafe fn rt31(&self);
    unsafe fn rt32(&self);
    unsafe fn rt33(&self);
    unsafe fn rt34(&self);
    unsafe fn rt35(&self);
    unsafe fn rt36(&self);
    unsafe fn rt37(&self);
    unsafe fn rt38(&self);
    unsafe fn rt39(&self);
    unsafe fn rt40(&self);
    unsafe fn rt41(&self);
    unsafe fn rt42(&self);
    unsafe fn clear(&self, clear_color: *const winapi::um::d2d1::D2D1_COLOR_F);
    unsafe fn begin_draw(&self);
    unsafe fn end_draw(
        &self,
        tag1: *mut winapi::um::d2d1::D2D1_TAG,
        tag2: *mut winapi::um::d2d1::D2D1_TAG,
    );
    unsafe fn rt46(&self);
    unsafe fn set_dpi(&self, dpix: f32, dpiy: f32);
    unsafe fn rt48(&self);
    unsafe fn rt49(&self);
    unsafe fn rt50(&self);
    unsafe fn rt51(&self);
    unsafe fn rt52(&self);
}

#[com_interface("2cd90691-12e2-11dc-9fed-001143a055f9")]
pub trait ID2D1Resource: IUnknown {
    unsafe fn r0(&self);
}

#[com_interface("db6f6ddb-ac77-4e88-8253-819df9bbf140")]
pub trait ID3D11Device: IUnknown {}

#[com_interface("54ec77fa-1377-44e6-8c32-88fd5f44c84c")]
pub trait IDXGIDevice: IDXGIObject {
    unsafe fn get_adapter(&self, adapter: *mut Option<ComPtr<dyn IDXGIAdapter>>) -> HRESULT;
    unsafe fn d2(&self);
    unsafe fn d3(&self);
    unsafe fn d4(&self);
}

#[com_interface("aec22fb8-76f3-4639-9be0-28eb43a67a2e")]
pub trait IDXGIObject: IUnknown {
    unsafe fn o0(&self);
    unsafe fn o1(&self);
    unsafe fn o2(&self);
    unsafe fn get_parent(
        &self,
        refid: winapi::shared::guiddef::REFIID,
        pparent: *mut *mut std::ffi::c_void,
    ) -> HRESULT;
}

#[com_interface("790a45f7-0d42-4876-983a-0a55cfe6f4aa")]
pub trait IDXGISwapChain1: IDXGISwapChain {}

#[com_interface("310d36a0-d2e7-4c0a-aa04-6a9d23b8886a")]
pub trait IDXGISwapChain: IDXGIDeviceSubObject {
    unsafe fn present(
        &self,
        sync_interval: winapi::shared::minwindef::UINT,
        flags: winapi::shared::minwindef::UINT,
    ) -> HRESULT;
    unsafe fn get_buffer(
        &self,
        buffer: winapi::shared::minwindef::UINT,
        riid: winapi::shared::guiddef::REFIID,
        pp_surface: *mut *mut std::ffi::c_void,
    ) -> HRESULT;
}

#[com_interface("3d3e0379-f9de-4d58-bb6c-18d62992f1a6")]
pub trait IDXGIDeviceSubObject: IDXGIObject {
    unsafe fn so0(&self);
}

#[com_interface("2411e7e1-12ac-4ccf-bd14-9798e8534dc0")]
pub trait IDXGIAdapter: IDXGIObject {
    unsafe fn a0(&self);
    unsafe fn a1(&self);
    unsafe fn a2(&self);
}

#[com_interface("ae02eedb-c735-4690-8d52-5a8dc20213aa")]
pub trait IDXGIOutput: IDXGIObject {}

#[com_interface("cafcb56c-6ac3-4889-bf47-9e23bbd260ec")]
pub trait IDXGISurface: IDXGIDeviceSubObject {}

#[com_interface("a898a84c-3873-4588-b08b-ebbf978df041")]
pub trait ID2D1Bitmap1: ID2D1Bitmap {}

#[com_interface("a2296057-ea42-4099-983b-539fb6505426")]
pub trait ID2D1Bitmap: ID2D1Image {}

#[com_interface("65019f75-8da2-497c-b32c-dfa34e48ede6")]
pub trait ID2D1Image: ID2D1Resource {}

#[com_interface("9169896C-AC8D-4e7d-94E5-67FA4DC2F2E8")]
pub trait IUIAnimationManager: IUnknown {
    unsafe fn a0(&self);
    unsafe fn a1(&self);
    unsafe fn a2(&self);
    unsafe fn a3(&self);
    unsafe fn a4(&self);
    //6
    unsafe fn update(&self, time_now: UI_ANIMATION_SECONDS) -> HRESULT;
}

type UI_ANIMATION_SECONDS = f64;

#[com_interface("10a72a66-e91c-43f4-993f-ddf4b82b0b4a")]
pub trait ID2D1StrokeStyle1: ID2D1StrokeStyle {}

#[com_interface("2cd9069d-12e2-11dc-9fed-001143a055f9")]
pub trait ID2D1StrokeStyle: ID2D1Resource {}
