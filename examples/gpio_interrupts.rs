#![no_main]
#![no_std]

use panic_semihosting as _;

use stm32f3xx_hal as hal;

use core::cell::RefCell;
use cortex_m::asm;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use hal::gpio::{gpioa, gpioe, Edge, ExtiPin, Input, Output, PullDown, PushPull};
use hal::interrupt;
use hal::pac;
use hal::pac::{Interrupt, NVIC};
use hal::prelude::*;

type LedPin = gpioe::PE9<Output<PushPull>>;
static LED: Mutex<RefCell<Option<LedPin>>> = Mutex::new(RefCell::new(None));

type ButtonPin = gpioa::PA0<Input<PullDown>>;
static BUTTON: Mutex<RefCell<Option<ButtonPin>>> = Mutex::new(RefCell::new(None));

// When the user button is pressed. The north LED with toggle.
#[entry]
fn main() -> ! {
    // Getting access to registers we will need for configuration.
    let device_peripherals = pac::Peripherals::take().unwrap();
    let mut rcc = device_peripherals.RCC.constrain();
    let mut syscfg = device_peripherals.SYSCFG;
    let mut exti = device_peripherals.EXTI;
    let mut gpioe = device_peripherals.GPIOE.split(&mut rcc.ahb);
    let mut gpioa = device_peripherals.GPIOA.split(&mut rcc.ahb);

    let mut led = gpioe
        .pe9
        .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);
    // Turn the led on so we know the configuration step occurred.
    led.toggle().expect("unable to toggle led in configuration");

    // Move the ownership of the led to the global LED
    cortex_m::interrupt::free(|cs| *LED.borrow(cs).borrow_mut() = Some(led));

    // Configuring the user button to trigger an interrupt when the button is pressed.
    let mut user_button = gpioa
        .pa0
        .into_pull_down_input(&mut gpioa.moder, &mut gpioa.pupdr);
    user_button.make_interrupt_source(&mut syscfg);
    user_button.trigger_on_edge(&mut exti, Edge::RISING);
    user_button.enable_interrupt(&mut exti);
    // Moving ownership to the global BUTTON so we can clear the interrupt pending bit.
    cortex_m::interrupt::free(|cs| *BUTTON.borrow(cs).borrow_mut() = Some(user_button));

    unsafe { NVIC::unmask(Interrupt::EXTI0) }

    loop {
        asm::wfi();
    }
}

// Button Pressed interrupt.
// The exti# maps to the pin number that is being used as an external interrupt.
// See page 295 of the stm32f303 reference manual for proof:
// http://www.st.com/resource/en/reference_manual/dm00043574.pdf
//
// This may be called more than once per button press from the user since the button may not be debounced.
#[interrupt]
fn EXTI0() {
    cortex_m::interrupt::free(|cs| {
        // Toggle the LED
        LED.borrow(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .toggle()
            .unwrap();

        // Clear the interrupt pending bit so we don't infinitely call this routine
        BUTTON
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .clear_interrupt_pending_bit();
    })
}
