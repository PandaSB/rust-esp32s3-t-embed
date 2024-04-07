use core::convert::TryInto;

use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
//use embedded_graphics::{draw_target::DrawTarget, pixelcolor::Rgb565, prelude::*};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use esp_idf_hal::{
    delay::{Ets, FreeRtos}, gpio::*, spi::{config::Config, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SPI2, SPI3}
};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};

use display_interface_spi::SPIInterfaceNoCS;
use mipidsi::Builder;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use rotary_encoder_hal::{Direction, Rotary};


use log::info;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PWD");

static FLAG: AtomicBool =  AtomicBool::new(false);

fn gpio_int_callback() {
    // Assert FLAG indicating a press button happened
    FLAG.store(true, Ordering::Relaxed);
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();


    let mut power = PinDriver::output(unsafe { Gpio46::new() })?;
    power.set_high().unwrap() ; 

    // define pinout 
    let _io17 = PinDriver::output(unsafe { Gpio17::new() })?;
    let _io38 = PinDriver::output(unsafe { Gpio38::new() })?;
    let _io40 = PinDriver::output(unsafe { Gpio40::new() })?;
    let _io41 = PinDriver::output(unsafe { Gpio41::new() })?;
    let rota  = PinDriver::input(unsafe { Gpio2::new() })?;
    let rotb  = PinDriver::input(unsafe { Gpio1::new() })?;
    
    let mut rots = PinDriver::input(unsafe { Gpio0::new() })?;

    let mut io16 = PinDriver::output(unsafe { Gpio16::new() })?;


    let rst = PinDriver::input_output_od(unsafe { Gpio9::new() })?;
    let dc = PinDriver::input_output_od(unsafe { Gpio13::new() })?;
    let mut bl = PinDriver::output(unsafe { Gpio15::new() })?;

    let cs = unsafe { Gpio10::new()};
    let sclk = unsafe { Gpio12::new() };
    let sdo = unsafe { Gpio11::new() };
    let spi2 = unsafe {SPI2::new()} ; 
    let spi3 = unsafe {SPI3::new()} ; 

    let mut delay = Ets;

    let _sda = unsafe { Gpio18::new() };
    let _scl = unsafe { Gpio8::new() };

    let aclk = unsafe { Gpio45::new() };
    let adi = unsafe { Gpio42::new() };


    bl.set_high().unwrap() ; 
    
    let mut enc = Rotary::new(rota, rotb);
    let mut pos: isize = 0 ; 
    let mut count: isize = 0 ; 

    let _ = rots.set_pull(Pull::Up);
    let _ = rots.set_interrupt_type(InterruptType::PosEdge) ; 
    unsafe { rots.subscribe(gpio_int_callback).unwrap() }
    rots.enable_interrupt().unwrap();



    let spi2 = SpiDriver::new(
        spi2,
        sclk,
        sdo,
        None::<AnyIOPin>,
        &SpiDriverConfig::new(),
    )?;

    let spi3 = SpiDriver::new(
        spi3,
        aclk,
        adi,
        None::<AnyIOPin>,
        &SpiDriverConfig::new(),
    )?;
    let spi2 = SpiDeviceDriver::new(spi2, Some(cs), &Config::new())?;
    let _spi3 = SpiDeviceDriver::new(spi3, None::<AnyIOPin>, &Config::new())?;

    let di =  SPIInterfaceNoCS::new(spi2,dc); 


    let mut display = Builder::st7789(di)
        .with_display_size(170,320)

        .init(&mut delay, Some(rst)) 
        .unwrap(); 
    
    let _ = display.clear(Rgb565::RED); 



    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    connect_wifi(&mut wifi)?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    println!("Wifi DHCP info: {:?}", ip_info);

   

    std::thread::sleep(core::time::Duration::from_secs(5));

    loop {
        match enc.update().unwrap()
        {
            Direction::Clockwise => {
                pos += 1;
                println!("position : {:?}", pos);
            }           
            Direction::CounterClockwise => {
                pos -=1;
                println!("position : {:?}", pos);
            }
            Direction::None => {}
        }
        if FLAG.load(Ordering::Relaxed) {
            FLAG.store(false,Ordering::Relaxed ) ; 
            count +=1 ; 
            println!("Press Count {}", count);
            rots.enable_interrupt().unwrap();
        }

        io16.set_low().unwrap() ; 
        FreeRtos::delay_ms(1);
        io16.set_high().unwrap();
        FreeRtos::delay_ms(1)

    }

    Ok(())
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(())
}
