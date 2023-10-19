#[macro_export]
macro_rules! magic {
    ($path:path) => {
        use core::panic::PanicInfo;
        use lithium::*;

        bootloader::entry_point!(kernel_main);

        fn kernel_main(boot_info: &'static bootloader::BootInfo) -> ! {
            $crate::init(boot_info);

            let f: fn() = $path;
            f();

            loop {
                x86_64::instructions::hlt();
            }
        }

        #[panic_handler]
        fn panic(info: &PanicInfo) -> ! {
            err!("{}", info);
            loop {
                x86_64::instructions::hlt();
            }
        }
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::sys::serial::print_fmt(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    () => ({
        $crate::print!("\n");
    });
    ($($arg:tt)*) => ({
        $crate::print!("{}\n", format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        let csi_green = $crate::sys::console::Style::new().foreground($crate::sys::console::Color::Green);
        let reset_color = $crate::sys::console::Style::reset();

        $crate::print!("{}[ INFO ]{} ", csi_green, reset_color);
        $crate::print!("{}", format_args!($($arg)*));
        $crate::println!();
    });
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        let csi_yellow = $crate::sys::console::Style::new().foreground($crate::sys::console::Color::Yellow);
        let reset_color = $crate::sys::console::Style::reset();

        $crate::print!("{}[ DEBUG ]{} ", csi_yellow, reset_color);
        $crate::print!("{}", format_args!($($arg)*));
        $crate::println!();
    });
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => ({
        let csi_red = $crate::sys::console::Style::new().foreground($crate::sys::console::Color::Red);
        let reset_color = $crate::sys::console::Style ::reset();

        $crate::print!("{}[ ERROR ]{} ", csi_red, reset_color);
        $crate::print!("{}", format_args!($($arg)*));
        $crate::println!();
    });
}