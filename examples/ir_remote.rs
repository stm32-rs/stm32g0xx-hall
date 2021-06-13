#![no_std]
#![no_main]
// #![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

use hal::exti::Event;
use hal::gpio::gpiob::PB9;
use hal::gpio::Analog;
use hal::gpio::SignalEdge;
use hal::prelude::*;
use hal::rcc;
use hal::stm32;
use hal::time::*;
use hal::timer::irtim::IrTransmitter;
use hal::timer::Timer;
use infrared::protocols::nec::NecCommand;
use infrared::{protocols::Nec, Sender};
use rtic::app;

const IR_SAMPLERATE: Hertz = Hertz(20_000);
const STROBE_COMMAND: NecCommand = NecCommand {
    addr: 0,
    cmd: 15,
    repeat: false,
};

#[app(device = hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        timer: Timer<stm32::TIM15>,
        transmitter: Sender<Nec, IrTransmitter<stm32::TIM16, PB9<Analog>>>,
        exti: stm32::EXTI,
    }

    #[init]
    fn init(mut ctx: init::Context) -> init::LateResources {
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::pll());

        let gpiob = ctx.device.GPIOB.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        gpioc.pc13.listen(SignalEdge::Falling, &mut ctx.device.EXTI);

        let mut timer = ctx.device.TIM15.timer(&mut rcc);
        timer.start(IR_SAMPLERATE);
        timer.listen();

        let ir_phy =
            ctx.device
                .TIM17
                .ir_transmitter(38.khz(), ctx.device.TIM16, gpiob.pb9, &mut rcc);
        let transmitter = Sender::new(IR_SAMPLERATE.0, ir_phy);

        init::LateResources {
            timer,
            transmitter,
            exti: ctx.device.EXTI,
        }
    }

    #[task(binds = TIM15, resources = [timer, transmitter])]
    fn timer_tick(ctx: timer_tick::Context) {
        ctx.resources.transmitter.tick();
        ctx.resources.timer.clear_irq();
    }

    #[task(binds = EXTI4_15, resources = [exti, transmitter])]
    fn button_click(ctx: button_click::Context) {
        ctx.resources
            .transmitter
            .load(&STROBE_COMMAND)
            .expect("failed to send IR command");
        ctx.resources.exti.unpend(Event::GPIO13);
    }
};
